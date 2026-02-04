use crate::state::whitelist::Whitelist;
use anchor_lang::{
    prelude::*, 
    system_program};


#[derive(Accounts)]
pub struct AddToWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: User account
    #[account(mut)]
    pub user: AccountInfo<'info>,
    #[account(
        init,
        payer = admin,
        space = 8,
        seeds = [b"whitelist", user.key().as_ref()],
        bump,
    )]
    pub whitelist_account: Account<'info, Whitelist>,
    pub system_program: Program<'info, System>,
}

impl<'info> AddToWhitelist<'info> {
    pub fn add_to_whitelist(&mut self) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct RemoveFromWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: User account
    #[account(mut)]
    pub user: AccountInfo<'info>,
    #[account(
        mut,
        close = admin,
        seeds = [b"whitelist", user.key().as_ref()],
        bump,
    )]
    pub whitelist_account: Account<'info, Whitelist>,
    pub system_program: Program<'info, System>,
}

impl<'info> RemoveFromWhitelist<'info> {
    pub fn remove_from_whitelist(&mut self) -> Result<()> {
        Ok(())
    }
}