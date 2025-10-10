// example of a poker game, doesnt actually work as one
use anchor_lang::prelude::*;
use card_shuffler_client::{
    get_card_game_pda, CardGame, CARD_SHUFFLER_PROGRAM_ID,
};

declare_id!("56Hf5PcrNpJ9z94MczM2iWymqP8oR7pxAtRirveQqCPv");

#[program]
pub mod example {
    use super::*;

    /// Create a poker game session
    /// consumes the card_shuffler_client crate
    pub fn create_game_session(
        ctx: Context<CreateGameSession>,
        game_id: u64,
    ) -> Result<()> {
        let game_session = &mut ctx.accounts.game_session;
        game_session.game_id = game_id;
        game_session.player = ctx.accounts.player.key();
        game_session.bump = ctx.bumps.game_session;
        game_session.game_state = GameState::WaitingToShuffle;

        // lculate the expected card_game PDA from card_shuffler
        let (card_game_pda, _) = get_card_game_pda(game_id);

        msg!("Created game session");
        msg!("  Game ID: {}", game_id);
        msg!("  Player: {}", ctx.accounts.player.key());
        msg!("  Card Game PDA (card_shuffler): {}", card_game_pda);

        Ok(())
    }

    /// Start the poker hand - marks game ready for card operations
    /// will need to call arcium job
    pub fn start_hand(ctx: Context<StartHand>) -> Result<()> {
        let game_session = &mut ctx.accounts.game_session;

        require!(
            game_session.game_state == GameState::WaitingToShuffle,
            PokerError::WrongGameState
        );

        game_session.game_state = GameState::ShufflingDeck;
        game_session.hand_number += 1;

        msg!("Hand #{} started - ready to shuffle", game_session.hand_number);
        msg!("TypeScript should now call card_shuffler.shuffleAndDeal()");

        Ok(())
    }

    /// Deal hole cards - updates game state after shuffle completes
    /// will need to call arcium job
    pub fn deal_hole_cards(ctx: Context<DealHoleCards>) -> Result<()> {
        let game_session = &mut ctx.accounts.game_session;

        require!(
            game_session.game_state == GameState::ShufflingDeck,
            PokerError::WrongGameState
        );

        game_session.game_state = GameState::HoleCardsDealt;

        msg!("Deck shuffled - ready to deal hole cards");
        msg!("TypeScript should now call card_shuffler.storeHoleCards()");

        Ok(())
    }

    /// Reveal community cards (flop, turn, or river)
    /// Reads CardGame to verify cards were revealed
    pub fn reveal_community_cards(
        ctx: Context<RevealCommunityCards>,
        _num_cards: u8,
    ) -> Result<()> {
        let game_session = &mut ctx.accounts.game_session;
        let card_game = &ctx.accounts.card_game;

        require!(
            game_session.game_state == GameState::HoleCardsDealt
            || game_session.game_state == GameState::Flop
            || game_session.game_state == GameState::Turn,
            PokerError::WrongGameState
        );

        // Update game state based on how many community cards are revealed
        game_session.game_state = match card_game.community_cards_size {
            3 => GameState::Flop,
            4 => GameState::Turn,
            5 => GameState::River,
            _ => return Err(PokerError::InvalidCommunityCards.into()),
        };

        msg!("Community cards revealed:");
        for i in 0..card_game.community_cards_size {
            msg!("  Card {}: {}", i, card_game.community_cards[i as usize]);
        }
        msg!("Game state: {:?}", game_session.game_state);

        Ok(())
    }

    /// Query info about the poker game and card state
    /// This reads the CardGame account from card_shuffler program
    pub fn get_game_info(ctx: Context<GetGameInfo>) -> Result<()> {
        let game_session = &ctx.accounts.game_session;
        let card_game = &ctx.accounts.card_game;

        msg!("=== Game Session Info ===");
        msg!("  Game ID: {}", game_session.game_id);
        msg!("  Player: {}", game_session.player);
        msg!("  Hand Number: {}", game_session.hand_number);
        msg!("  Game State: {:?}", game_session.game_state);

        msg!("=== Card Game State (from card_shuffler) ===");
        msg!("  Hole cards size: {}", card_game.hole_cards_size);
        msg!("  Community cards size: {}", card_game.community_cards_size);
        msg!("  Cards dealt: {}", card_game.cards_dealt);

        // Show community cards
        if card_game.community_cards_size > 0 {
            msg!("  Community cards:");
            for i in 0..card_game.community_cards_size {
                msg!("    Card {}: {}", i, card_game.community_cards[i as usize]);
            }
        }

        emit!(GameInfoEvent {
            game_id: game_session.game_id,
            hand_number: game_session.hand_number,
            game_state: game_session.game_state,
            hole_cards_size: card_game.hole_cards_size,
            community_cards_size: card_game.community_cards_size,
            cards_dealt: card_game.cards_dealt,
        });

        Ok(())
    }

    /// End the current hand and reset for next hand
    /// will need to call arcium job
    pub fn end_hand(ctx: Context<EndHand>) -> Result<()> {
        let game_session = &mut ctx.accounts.game_session;

        require!(
            game_session.game_state == GameState::River,
            PokerError::WrongGameState
        );

        game_session.game_state = GameState::WaitingToShuffle;

        msg!("Hand complete - ready for next hand");
        msg!("TypeScript can call card_shuffler.changeHand() to deal new cards");

        Ok(())
    }

    /// Close a game session
    pub fn close_game_session(_ctx: Context<CloseGameSession>) -> Result<()> {
        msg!("Game session closed");
        Ok(())
    }
}

/// Account to track a poker game session
#[account]
#[derive(InitSpace)]
pub struct GameSession {
    pub game_id: u64,
    pub player: Pubkey,
    pub bump: u8,
    pub hand_number: u64,
    pub game_state: GameState,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum GameState {
    WaitingToShuffle,
    ShufflingDeck,
    HoleCardsDealt,
    Flop,
    Turn,
    River,
}

#[derive(Accounts)]
#[instruction(game_id: u64)]
pub struct CreateGameSession<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        init,
        payer = player,
        space = 8 + GameSession::INIT_SPACE,
        seeds = [b"game_session", player.key().as_ref(), game_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub game_session: Account<'info, GameSession>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StartHand<'info> {
    #[account(mut)]
    pub game_session: Account<'info, GameSession>,
}

#[derive(Accounts)]
pub struct DealHoleCards<'info> {
    #[account(mut)]
    pub game_session: Account<'info, GameSession>,
}

#[derive(Accounts)]
pub struct RevealCommunityCards<'info> {
    #[account(mut)]
    pub game_session: Account<'info, GameSession>,

    /// The CardGame account from the card_shuffler program
    #[account(
        seeds = [b"card_game", game_session.game_id.to_le_bytes().as_ref()],
        bump,
        seeds::program = CARD_SHUFFLER_PROGRAM_ID,
    )]
    pub card_game: Account<'info, CardGame>,
}

#[derive(Accounts)]
pub struct GetGameInfo<'info> {
    pub game_session: Account<'info, GameSession>,

    /// The CardGame account from the card_shuffler program
    #[account(
        seeds = [b"card_game", game_session.game_id.to_le_bytes().as_ref()],
        bump,
        seeds::program = CARD_SHUFFLER_PROGRAM_ID,
    )]
    pub card_game: Account<'info, CardGame>,
}

#[derive(Accounts)]
pub struct EndHand<'info> {
    #[account(mut)]
    pub game_session: Account<'info, GameSession>,
}

#[derive(Accounts)]
pub struct CloseGameSession<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        mut,
        close = player,
        has_one = player,
    )]
    pub game_session: Account<'info, GameSession>,
}

#[event]
pub struct GameInfoEvent {
    pub game_id: u64,
    pub hand_number: u64,
    pub game_state: GameState,
    pub hole_cards_size: u8,
    pub community_cards_size: u8,
    pub cards_dealt: u8,
}

#[error_code]
pub enum PokerError {
    #[msg("Wrong game state for this operation")]
    WrongGameState,
    #[msg("Invalid number of community cards")]
    InvalidCommunityCards,
}
