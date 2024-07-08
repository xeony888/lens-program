use anchor_lang::prelude::*;
declare_id!("4JwBm4nCWHmxRrtc2bSeM6idcsgoBkNdq8VNyFdYYVbR");

const CREATOR: &str = "58V6myLoy5EVJA3U2wPdRDMUXpkwg8Vfw5b6fHqi2mej";
const LAMPORTS_PER_SEC: u64 = 100;
#[program]
pub mod lens_payment {
    use super::*;
    pub fn initialize_payment_account(ctx: Context<InitializePaymentAccount>, id: String, level: u8) -> Result<()> {
        if level < 1 {
            return Err(CustomError::InvalidLevel.into())
        }
        let time = Clock::get()?.unix_timestamp as u64;
        ctx.accounts.payment_account.until = time;
        ctx.accounts.payment_account.level = level;
        ctx.accounts.payment_account.authority = ctx.accounts.signer.key();
        ctx.accounts.payment_account.id = id;
        Ok(())
    }
    pub fn pay(ctx: Context<Pay>, id: String, level: u8, amount: u64) -> Result<()> {
        let lamports_needed = (amount * level as u64) * LAMPORTS_PER_SEC;
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.signer.to_account_info(),
                    to: ctx.accounts.payment_holder_account.to_account_info(),
                }
            ),
            lamports_needed
        )?;
        let time = Clock::get()?.unix_timestamp as u64;
        if ctx.accounts.payment_account.until < time {
            ctx.accounts.payment_account.until = time + amount;
        } else {
            ctx.accounts.payment_account.until += amount;
        }
        Ok(())
    }
    pub fn cancel(ctx: Context<Cancel>, id: String, level: u8, amount: u64) -> Result<()> {
        let payback = (amount * level as u64) * LAMPORTS_PER_SEC;
        let time = Clock::get()?.unix_timestamp as u64;
        if ctx.accounts.payment_account.until - amount < time {
            return Err(CustomError::CannotCancelPast.into())
        }
        ctx.accounts.payment_account.until = ctx.accounts.payment_account.until.checked_sub(amount).ok_or(CustomError::OverflowError)?;
        **ctx.accounts.payment_holder_account.try_borrow_mut_lamports()? -= payback;
        **ctx.accounts.signer.try_borrow_mut_lamports()? += payback;
        Ok(())
    }
    pub fn withdraw(ctx: Context<Withdraw>, id: String, level: u8) -> Result<()> {
        let supposed = CREATOR.parse::<Pubkey>().unwrap();
        // if ctx.accounts.signer.key() != supposed.key() {
        //     return Err(CustomError::Unauthorized.into())
        // }
        let time = Clock::get()?.unix_timestamp as u64;
        let min_rent = Rent::get()?.minimum_balance(8);
        let transfer = if ctx.accounts.payment_account.until > time {
            let remaining = (ctx.accounts.payment_account.until - time) * (level as u64) * LAMPORTS_PER_SEC;
            msg!("{}, {}, {}", ctx.accounts.payment_holder_account.get_lamports(), remaining, min_rent);
            ctx.accounts.payment_holder_account.get_lamports() - remaining - min_rent
        } else {
            ctx.accounts.payment_holder_account.get_lamports() - min_rent // lamport balance minus minimum required
        };
        **ctx.accounts.payment_holder_account.try_borrow_mut_lamports()? -= transfer;
        **ctx.accounts.signer.try_borrow_mut_lamports()? += transfer;
        Ok(())
    }
}
#[error_code]
pub enum CustomError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Cannot cancel past")]
    CannotCancelPast,
    #[msg("Level cannot be less than 1")]
    InvalidLevel,
    #[msg("Overflow error")]
    OverflowError
}

#[account]
pub struct PaymentAccount {
    id: String,
    authority: Pubkey,
    until: u64,
    level: u8,
}
#[derive(Accounts)]
#[instruction(id: String, level: u8)]
pub struct InitializePaymentAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        seeds = [b"payment", id.as_bytes(), level.to_le_bytes().as_ref()],
        bump,
        payer = signer,
        space = 8 + 4 + id.len() + 32 + 8 + 1,
    )]
    pub payment_account: Account<'info, PaymentAccount>,
    #[account(
        init,
        seeds = [b"holder", id.as_bytes(), level.to_le_bytes().as_ref()],
        bump,
        payer = signer,
        space = 8
    )]
    /// CHECK: 
    pub payment_holder_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(id: String, level: u8)]
pub struct Pay<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"payment", id.as_bytes(), level.to_le_bytes().as_ref()],
        bump
    )]
    pub payment_account: Account<'info, PaymentAccount>,
    #[account(
        mut,
        seeds = [b"holder", id.as_bytes(), level.to_le_bytes().as_ref()],
        bump,
    )]
    /// CHECK: 
    pub payment_holder_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
#[instruction(id: String, level: u8)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"payment", id.as_bytes(), level.to_le_bytes().as_ref()],
        bump
    )]
    pub payment_account: Account<'info, PaymentAccount>,
    #[account(
        mut,
        seeds = [b"holder", id.as_bytes(), level.to_le_bytes().as_ref()],
        bump,
    )]
    /// CHECK: 
    pub payment_holder_account: AccountInfo<'info>,
}
#[derive(Accounts)]
#[instruction(id: String, level: u8)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"payment", id.as_bytes(), level.to_le_bytes().as_ref()],
        bump
    )]
    pub payment_account: Account<'info, PaymentAccount>,
    #[account(
        mut,
        seeds = [b"holder", id.as_bytes(), level.to_le_bytes().as_ref()],
        bump,
    )]
    /// CHECK: 
    pub payment_holder_account: AccountInfo<'info>,
}
