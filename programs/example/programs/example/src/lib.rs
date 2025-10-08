use anchor_lang::prelude::*;

declare_id!("GnbBa6jp5XJqhXWd9Eae83JCixSwbAG8sR2qphzdKz8r");

#[program]
pub mod example {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
