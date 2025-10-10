use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::CallbackAccount;

const COMP_DEF_OFFSET_SHUFFLE_AND_DEAL: u32 = comp_def_offset("shuffle_and_deal_deck");
const COMP_DEF_OFFSET_STORE_HOLE_CARDS: u32 = comp_def_offset("store_hole_cards");
const COMP_DEF_OFFSET_REVEAL_COMMUNITY: u32 = comp_def_offset("reveal_community_cards");
const COMP_DEF_OFFSET_CHANGE_HAND: u32 = comp_def_offset("change_hand");

declare_id!("DQxanaqqWcTYvVhrKbeoY6q52NrGksWBL6vSbuVipnS7");

#[arcium_program]
pub mod card_shuffler {
    use super::*;

    /// Initializes the computation definition for shuffling and dealing cards
    pub fn init_shuffle_and_deal_comp_def(
        ctx: Context<InitShuffleAndDealCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }

    /// Initializes a new card game and shuffles the deck
    /// Deals initial hole cards to the player encrypted with their public key
    pub fn initialize_card_game(
        ctx: Context<InitializeCardGame>,
        computation_offset: u64,
        game_id: u64,
        mxe_nonce: u128,
        client_pubkey: [u8; 32],
        client_nonce: u128,
        num_hole_cards: u8,
    ) -> Result<()> {
        let card_game = &mut ctx.accounts.card_game;
        card_game.bump = ctx.bumps.card_game;
        card_game.game_id = game_id;
        card_game.player_pubkey = ctx.accounts.payer.key();
        card_game.player_enc_pubkey = client_pubkey;
        card_game.deck = [[0; 32]; 3];
        card_game.deck_nonce = 0;
        card_game.hole_cards = [0; 32];
        card_game.hole_cards_nonce = 0;
        card_game.hole_cards_size = 0;
        card_game.community_cards = [53; 5];
        card_game.community_cards_size = 0;
        card_game.cards_dealt = 0;

        // Queue the shuffle and deal computation
        let args = vec![
            Argument::PlaintextU128(mxe_nonce),
            Argument::ArcisPubkey(client_pubkey),
            Argument::PlaintextU128(client_nonce),
            Argument::PlaintextU8(num_hole_cards),
        ];

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![ShuffleAndDealDeckCallback::callback_ix(&[CallbackAccount {
                pubkey: ctx.accounts.card_game.key(),
                is_writable: true,
            }])],
        )?;

        Ok(())
    }

    /// Callback handler
    #[arcium_callback(encrypted_ix = "shuffle_and_deal_deck")]
    pub fn shuffle_and_deal_deck_callback(
        ctx: Context<ShuffleAndDealDeckCallback>,
        output: ComputationOutputs<ShuffleAndDealDeckOutput>,
    ) -> Result<()> {
        let o = match output {
            ComputationOutputs::Success(ShuffleAndDealDeckOutput {
                field_0:
                    ShuffleAndDealDeckOutputStruct0 {
                        field_0: deck,
                        field_1: hole_cards,
                        field_2: num_dealt,
                    },
            }) => (deck, hole_cards, num_dealt),
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let deck_nonce = o.0.nonce;
        let deck: [[u8; 32]; 3] = o.0.ciphertexts;

        let hole_cards_nonce = o.1.nonce;
        let hole_cards: [u8; 32] = o.1.ciphertexts[0];

        let num_dealt: u8 = o.2;

        let card_game = &mut ctx.accounts.card_game;
        card_game.deck = deck;
        card_game.deck_nonce = deck_nonce;
        card_game.hole_cards = hole_cards;
        card_game.hole_cards_nonce = hole_cards_nonce;
        card_game.hole_cards_size = num_dealt;
        card_game.cards_dealt = num_dealt;

        emit!(DeckShuffledEvent {
            game_id: card_game.game_id,
            hole_cards,
            hole_cards_nonce,
            num_hole_cards: num_dealt,
        });

        Ok(())
    }

    /// Initializes the computation definition for storing hole cards
    pub fn init_store_hole_cards_comp_def(
        ctx: Context<InitStoreHoleCardsCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }

    /// Stores additional encrypted hole cards
    pub fn store_hole_cards(
        ctx: Context<StoreHoleCards>,
        computation_offset: u64,
        _game_id: u64,
        num_new_cards: u8,
    ) -> Result<()> {
        let card_game = &ctx.accounts.card_game;

        let args = vec![
            // Deck
            Argument::PlaintextU128(card_game.deck_nonce),
            Argument::Account(card_game.key(), 8, 32 * 3),
            // Existing hand
            Argument::ArcisPubkey(card_game.player_enc_pubkey),
            Argument::PlaintextU128(card_game.hole_cards_nonce),
            Argument::Account(card_game.key(), 8 + 32 * 3, 32),
            // Hand size
            Argument::PlaintextU8(card_game.hole_cards_size),
            // New cards to add
            Argument::PlaintextU8(num_new_cards),
            // Cards already dealt
            Argument::PlaintextU8(card_game.cards_dealt),
        ];

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![StoreHoleCardsCallback::callback_ix(&[CallbackAccount {
                pubkey: card_game.key(),
                is_writable: true,
            }])],
        )?;

        Ok(())
    }

    /// Callback handler 
    #[arcium_callback(encrypted_ix = "store_hole_cards")]
    pub fn store_hole_cards_callback(
        ctx: Context<StoreHoleCardsCallback>,
        output: ComputationOutputs<StoreHoleCardsOutput>,
    ) -> Result<()> {
        let o = match output {
            ComputationOutputs::Success(StoreHoleCardsOutput {
                field_0:
                    StoreHoleCardsOutputStruct0 {
                        field_0: updated_hand,
                        field_1: new_hand_size,
                    },
            }) => (updated_hand, new_hand_size),
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let hole_cards_nonce = o.0.nonce;
        let hole_cards: [u8; 32] = o.0.ciphertexts[0];
        let new_size: u8 = o.1;

        let card_game = &mut ctx.accounts.card_game;
        let cards_added = new_size - card_game.hole_cards_size;

        card_game.hole_cards = hole_cards;
        card_game.hole_cards_nonce = hole_cards_nonce;
        card_game.hole_cards_size = new_size;
        card_game.cards_dealt += cards_added;

        emit!(HoleCardsStoredEvent {
            game_id: card_game.game_id,
            hole_cards,
            hole_cards_nonce,
            total_hole_cards: new_size,
        });

        Ok(())
    }

    /// Initializes the computation definition for revealing community cards
    pub fn init_reveal_community_comp_def(
        ctx: Context<InitRevealCommunityCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }

    /// Reveals community cards from the deck
    pub fn reveal_community_cards(
        ctx: Context<RevealCommunityCards>,
        computation_offset: u64,
        _game_id: u64,
        num_cards_to_reveal: u8,
    ) -> Result<()> {
        let card_game = &ctx.accounts.card_game;

        let args = vec![
            // Deck
            Argument::PlaintextU128(card_game.deck_nonce),
            Argument::Account(card_game.key(), 8, 32 * 3),
            // Number of cards to reveal
            Argument::PlaintextU8(num_cards_to_reveal),
            // Cards already dealt
            Argument::PlaintextU8(card_game.cards_dealt),
        ];

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![RevealCommunityCardsCallback::callback_ix(&[CallbackAccount {
                pubkey: card_game.key(),
                is_writable: true,
            }])],
        )?;

        Ok(())
    }

    /// Callback handler
    #[arcium_callback(encrypted_ix = "reveal_community_cards")]
    pub fn reveal_community_cards_callback(
        ctx: Context<RevealCommunityCardsCallback>,
        output: ComputationOutputs<RevealCommunityCardsOutput>,
    ) -> Result<()> {
        let o = match output {
            ComputationOutputs::Success(RevealCommunityCardsOutput {
                field_0:
                    RevealCommunityCardsOutputStruct0 {
                        field_0: cards,
                        field_1: num,
                    },
            }) => (cards, num),
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let community_cards = o.0;
        let num_revealed = o.1;

        let card_game = &mut ctx.accounts.card_game;
        card_game.community_cards = community_cards;
        card_game.community_cards_size = num_revealed;
        card_game.cards_dealt += num_revealed;

        emit!(CommunityCardsRevealedEvent {
            game_id: card_game.game_id,
            community_cards,
            num_revealed,
        });

        Ok(())
    }

    /// Initializes the computation definition for changing a hand
    pub fn init_change_hand_comp_def(ctx: Context<InitChangeHandCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }

    /// Resets/changes a player's hand for a new round
    pub fn change_hand(
        ctx: Context<ChangeHand>,
        computation_offset: u64,
        _game_id: u64,
        new_nonce: u128,
    ) -> Result<()> {
        let card_game = &ctx.accounts.card_game;

        let args = vec![
            Argument::ArcisPubkey(card_game.player_enc_pubkey),
            Argument::PlaintextU128(new_nonce),
        ];

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![ChangeHandCallback::callback_ix(&[CallbackAccount {
                pubkey: card_game.key(),
                is_writable: true,
            }])],
        )?;

        Ok(())
    }

    /// Callback handler 
    #[arcium_callback(encrypted_ix = "change_hand")]
    pub fn change_hand_callback(
        ctx: Context<ChangeHandCallback>,
        output: ComputationOutputs<ChangeHandOutput>,
    ) -> Result<()> {
        let new_hand = match output {
            ComputationOutputs::Success(ChangeHandOutput { field_0 }) => field_0,
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let new_nonce = new_hand.nonce;
        let new_hand_data: [u8; 32] = new_hand.ciphertexts[0];

        let card_game = &mut ctx.accounts.card_game;
        card_game.hole_cards = new_hand_data;
        card_game.hole_cards_nonce = new_nonce;
        card_game.hole_cards_size = 0;

        emit!(HandChangedEvent {
            game_id: card_game.game_id,
            new_hand: new_hand_data,
            new_nonce,
        });

        Ok(())
    }
}


#[queue_computation_accounts("shuffle_and_deal_deck", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, game_id: u64)]
pub struct InitializeCardGame<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!()
    )]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!()
    )]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SHUFFLE_AND_DEAL)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        init,
        payer = payer,
        space = 8 + CardGame::INIT_SPACE,
        seeds = [b"card_game".as_ref(), game_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub card_game: Account<'info, CardGame>,
}

#[callback_accounts("shuffle_and_deal_deck")]
#[derive(Accounts)]
pub struct ShuffleAndDealDeckCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SHUFFLE_AND_DEAL)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub card_game: Account<'info, CardGame>,
}

#[init_computation_definition_accounts("shuffle_and_deal_deck", payer)]
#[derive(Accounts)]
pub struct InitShuffleAndDealCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("store_hole_cards", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct StoreHoleCards<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!()
    )]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!()
    )]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_STORE_HOLE_CARDS)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        mut,
        seeds = [b"card_game".as_ref(), _game_id.to_le_bytes().as_ref()],
        bump = card_game.bump,
    )]
    pub card_game: Account<'info, CardGame>,
}

#[callback_accounts("store_hole_cards")]
#[derive(Accounts)]
pub struct StoreHoleCardsCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_STORE_HOLE_CARDS)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub card_game: Account<'info, CardGame>,
}

#[init_computation_definition_accounts("store_hole_cards", payer)]
#[derive(Accounts)]
pub struct InitStoreHoleCardsCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("reveal_community_cards", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct RevealCommunityCards<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!()
    )]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!()
    )]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_REVEAL_COMMUNITY)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        mut,
        seeds = [b"card_game".as_ref(), _game_id.to_le_bytes().as_ref()],
        bump = card_game.bump,
    )]
    pub card_game: Account<'info, CardGame>,
}

#[callback_accounts("reveal_community_cards")]
#[derive(Accounts)]
pub struct RevealCommunityCardsCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_REVEAL_COMMUNITY)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub card_game: Account<'info, CardGame>,
}

#[init_computation_definition_accounts("reveal_community_cards", payer)]
#[derive(Accounts)]
pub struct InitRevealCommunityCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("change_hand", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct ChangeHand<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!()
    )]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!()
    )]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_CHANGE_HAND)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        mut,
        seeds = [b"card_game".as_ref(), _game_id.to_le_bytes().as_ref()],
        bump = card_game.bump,
    )]
    pub card_game: Account<'info, CardGame>,
}

#[callback_accounts("change_hand")]
#[derive(Accounts)]
pub struct ChangeHandCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_CHANGE_HAND)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub card_game: Account<'info, CardGame>,
}

#[init_computation_definition_accounts("change_hand", payer)]
#[derive(Accounts)]
pub struct InitChangeHandCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

/// Represents a card game session with encrypted deck and hands
#[account]
#[derive(InitSpace)]
pub struct CardGame {
    /// Encrypted deck split into 3 chunks (52 cards encoded in base-64)
    pub deck: [[u8; 32]; 3],
    /// Cryptographic nonce for deck encryption
    pub deck_nonce: u128,
    /// Player's encrypted hole cards
    pub hole_cards: [u8; 32],
    /// Cryptographic nonce for hole cards encryption
    pub hole_cards_nonce: u128,
    /// Number of hole cards currently held
    pub hole_cards_size: u8,
    /// Revealed community cards (plaintext)
    pub community_cards: [u8; 5],
    /// Number of community cards revealed
    pub community_cards_size: u8,
    /// Total number of cards dealt from the deck
    pub cards_dealt: u8,
    /// Unique identifier for this game session
    pub game_id: u64,
    /// Solana public key of the player
    pub player_pubkey: Pubkey,
    /// Player's encryption public key for MPC operations
    pub player_enc_pubkey: [u8; 32],
    /// PDA bump seed
    pub bump: u8,
}

#[event]
pub struct DeckShuffledEvent {
    pub game_id: u64,
    pub hole_cards: [u8; 32],
    pub hole_cards_nonce: u128,
    pub num_hole_cards: u8,
}

#[event]
pub struct HoleCardsStoredEvent {
    pub game_id: u64,
    pub hole_cards: [u8; 32],
    pub hole_cards_nonce: u128,
    pub total_hole_cards: u8,
}

#[event]
pub struct CommunityCardsRevealedEvent {
    pub game_id: u64,
    pub community_cards: [u8; 5],
    pub num_revealed: u8,
}

#[event]
pub struct HandChangedEvent {
    pub game_id: u64,
    pub new_hand: [u8; 32],
    pub new_nonce: u128,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("Cluster not set")]
    ClusterNotSet,
}
