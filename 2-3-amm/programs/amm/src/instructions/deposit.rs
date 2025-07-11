use constant product curve:: ConstantProduct;
use anchor_spl::{associated_token::AssociatedToken, token::{mint_to, Mint, Token, TokenAccount}};
use crate::state::Config;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Deposit<'info>{
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,


    #[account(mut, seeds = [b"config", config.seed.to_le_bytes().as_ref()], bump = config.config_bump)]
    pub mint_lp: Account<'info, Mint>,
    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,



    #[account(
        mut, 
        payer = initializer,
        associated_token::mint = mint_x,
        associated_token::authority = user
    )]
    pub user_x: Account<'info, TokenAccount>,


    #[account(
        mut, 
        payer = initializer,
        associated_token::mint = mint_y,
        associated_token::authority = user
    )]
    pub user_y: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
        payer = user,
    )]
    pub user_lp: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>, 
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info>{

    pub fn deposit(&mut self, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
        require!(self.config.locked == false, AmmError::PoolLocked);
        require!(amount != 0, AmmError::InvalidAmount);

        let amounts: XYAmounts = match self.mint_lp.supply == 0 && self.vault_x.amount == 0 && self.vault_y.amount == 0 {
            true => XYAmounts { x: 0, y: 0, l: 0, a: 0, precision: 6 },
            false => ConstantProduct::xy_deposit_amounts_from_lp(
                self.vault_x.amount,
                self.vault_y.amount,
                self.mint_lp.supply,
                amount,
                6).unwrap(),
        };
        (amounts.x, amounts.y)

        require!(amounts.x <= max_x, AmmError::InvalidAmount);
        self.deposit_tokens(is_x: true, amount: x) ?;
        self.deposit_tokens (is_x: false, amount: y) ?;
        self.mint_lp_tokens (amount) ?;

        // Now use `amounts.x`, `amounts.y`, etc. as needed

        Ok(())
    }
    pub fn deposit_token(&mut self, is_x: bool, amount: u64) -> Result<()>{
        let (from, to): (AccountInfo<'_>, AccountInfo<'_>) = match is_x {
            true => (self.user_x.to_account_info(), self.vault_x.to_account_info()),
            false => (self.user_y.to_account_info(), self.vault_y.to_account_info()),
        };
        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = Transfer {
            from, to, authority: self.user.to_account_info(),
        };
        let ctx: CpiContext<Transfer> = CpiContext::new(cpi_program, cpi_accounts);
        transfer(ctx, amount)?;
    }

    pub fn mint_lp_tokens(&mut self, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = MintTo {
            mint: self.mint_lp.to_account_info(),
            to: self.user_lp.to_account_info(),
            authority: self.config.to_account_info(), // <-- Use the PDA if that's your mint authority
        };

        // Seeds for the PDA (adjust as needed for your program)
        let seeds: &[&[u8]] = &[
            b"config",
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ];
        let signer_seeds: &[&[&[u8]]] = &[seeds];

        let ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        mint_to(ctx, amount)?;
        Ok(())
    }
}