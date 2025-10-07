use anchor_lang::prelude::*;

declare_id!("6GvHYdjrQSFPb6TBSx49w5KUb37a3ZdyAwv9D2sHMGWD");

#[program]
pub mod card_shuffle {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
