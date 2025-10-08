use anchor_lang::prelude::*;

declare_id!("6GvHYdjrQSFPb6TBSx49w5KUb37a3ZdyAwv9D2sHMGWD");

#[program]
pub mod card_shuffle {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn set_deck_computation(ctx: Context<SetDeckComputation>, computation_offset: u64) -> Result<()> {
        emit!(DeckComputationSet { computation_offset });
        Ok(())
    }

    pub fn store_encrypted_hole_cards(
        ctx: Context<StoreEncryptedHoleCards>,
        owner: Pubkey,
        hand_ciphertext: Vec<u8>,
    ) -> Result<()> {
        emit!(HoleCardsStored { owner, num_bytes: hand_ciphertext.len() as u32 });
        Ok(())
    }

    pub fn reveal_community_cards(
        ctx: Context<RevealCommunityCards>,
        indices: Vec<u8>,
    ) -> Result<()> {
        emit!(CommunityCardsRequested { indices });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
pub struct SetDeckComputation<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
pub struct StoreEncryptedHoleCards<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
pub struct RevealCommunityCards<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[event]
pub struct DeckComputationSet {
    pub computation_offset: u64,
}

#[event]
pub struct HoleCardsStored {
    pub owner: Pubkey,
    pub num_bytes: u32,
}

#[event]
pub struct CommunityCardsRequested {
    pub indices: Vec<u8>,
}
