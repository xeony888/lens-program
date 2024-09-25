use anchor_lang::prelude::*;
use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
declare_id!("HqzzWgrEX3epRnsNMUNQPapFbAg8dLCSxskhCrz7NLnk");

const CREATOR: &str = "Ddi1GaugnX9yQz1WwK1b12m4o23rK1krZQMcnt2aNW97";
#[program]
pub mod lens_payment {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.global_data_account.fee_perc = 1;
        ctx.accounts.global_data_account.init_fee = LAMPORTS_PER_SOL / 10;
        Ok(())
    }
    pub fn modify_global_data(ctx: Context<ModifyGlobalData>, fee_perc: u64, init_fee: u64) -> Result<()> {
        ctx.accounts.global_data_account.fee_perc = fee_perc;
        ctx.accounts.global_data_account.init_fee = init_fee;
        Ok(())
    }
    pub fn create_payment_group(ctx: Context<CreatePaymentGroup>, id: u64, withdraw_authority: Pubkey, lamports_per_sec: u64, bypass: bool) -> Result<()> {
        if bypass {
            if !(CREATOR.parse::<Pubkey>().unwrap() == ctx.accounts.signer.key()) {
                return Err(CustomError::InvalidCreator.into())
            }
        } else {
            anchor_lang::system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::Transfer {
                        from: ctx.accounts.signer.to_account_info(),
                        to: ctx.accounts.global_holder_account.to_account_info()
                    }
                ),
                ctx.accounts.global_data_account.init_fee
            )?;  
        }
        ctx.accounts.payment_group_account.bypass = bypass;
        ctx.accounts.payment_group_account.withdraw_authority = withdraw_authority;
        ctx.accounts.payment_group_account.id = id;
        ctx.accounts.payment_group_account.modify_authority = ctx.accounts.signer.key();
        ctx.accounts.payment_group_account.lamports_per_sec = lamports_per_sec;
        Ok(())
    }
    pub fn modify_payment_group(ctx: Context<ModifyPaymentGroup>, id: u64, withdraw_authority: Pubkey, lamports_per_sec: u64) -> Result<()> {
        ctx.accounts.payment_group_account.withdraw_authority = withdraw_authority;
        ctx.accounts.payment_group_account.lamports_per_sec = lamports_per_sec;
        Ok(())
    }
    pub fn pay(ctx: Context<Pay>, group_id: u64, id: u64, level: u8, amount: u64) -> Result<()> {
        let lamports_needed = (amount * level as u64) * ctx.accounts.payment_group_account.lamports_per_sec;
        let fee = lamports_needed * ctx.accounts.global_data_account.fee_perc / 100;
        if !ctx.accounts.payment_group_account.bypass {
            anchor_lang::system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::Transfer {
                        from: ctx.accounts.signer.to_account_info(),
                        to: ctx.accounts.global_holder_account.to_account_info()
                    }
                ),
                fee
            )?;   
        }
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
        ctx.accounts.payment_account.authority = ctx.accounts.signer.key();
        ctx.accounts.payment_account.id = id;
        ctx.accounts.payment_account.group_id = group_id;
        ctx.accounts.payment_account.level = level;
        Ok(())
    }
    pub fn cancel(ctx: Context<Cancel>, group_id: u64, id: u64, level: u8, amount: u64) -> Result<()> {
        let payback = (amount * level as u64) * ctx.accounts.payment_group_account.lamports_per_sec;
        let time = Clock::get()?.unix_timestamp as u64;
        if ctx.accounts.payment_account.until - amount < time {
            return Err(CustomError::CannotCancelPast.into())
        }
        ctx.accounts.payment_account.until = ctx.accounts.payment_account.until.checked_sub(amount).ok_or(CustomError::OverflowError)?;
        **ctx.accounts.payment_holder_account.try_borrow_mut_lamports()? -= payback;
        **ctx.accounts.signer.try_borrow_mut_lamports()? += payback;
        Ok(())
    }
    pub fn withdraw(ctx: Context<Withdraw>, group_id: u64, id: u64, level: u8) -> Result<()> {
        let time = Clock::get()?.unix_timestamp as u64;
        let min_rent = Rent::get()?.minimum_balance(8);
        let transfer = if ctx.accounts.payment_account.until > time {
            let remaining = (ctx.accounts.payment_account.until - time) * (level as u64) * ctx.accounts.payment_group_account.lamports_per_sec;
            msg!("{}, {}, {}", ctx.accounts.payment_holder_account.get_lamports(), remaining, min_rent);
            ctx.accounts.payment_holder_account.get_lamports() - remaining - min_rent
        } else {
            msg!("nope");
            ctx.accounts.payment_holder_account.get_lamports() - min_rent // lamport balance minus minimum required
        };
        **ctx.accounts.payment_holder_account.try_borrow_mut_lamports()? -= transfer;
        **ctx.accounts.signer.try_borrow_mut_lamports()? += transfer;
        Ok(())
    }
    pub fn withdraw_program_funds(ctx: Context<WithdrawProgramFunds>) -> Result<()> {
        let min_rent = Rent::get()?.minimum_balance(8);
        let transfer = ctx.accounts.global_holder_account.get_lamports() - min_rent;
        **ctx.accounts.global_holder_account.try_borrow_mut_lamports()? -= transfer;
        **ctx.accounts.signer.try_borrow_mut_lamports()? += transfer;
        Ok(())
    }
}
#[error_code]
pub enum CustomError {
    #[msg("Invalid creator")]
    InvalidCreator,
    #[msg("Invalid modify authority")]
    InvalidModifyAuthority,
    #[msg("Invalid payment authority")]
    InvalidPaymentAuthority,
    #[msg("Cannot cancel past")]
    CannotCancelPast,
    #[msg("Overflow error")]
    OverflowError,
    #[msg("Invalid withdraw authority")]
    InvalidWithdrawAuthority
}
#[account]
pub struct GlobalDataAccount {
    pub fee_perc: u64,
    pub init_fee: u64,
}
#[account]
pub struct PaymentGroupAccount {
    id: u64,
    modify_authority: Pubkey,
    withdraw_authority: Pubkey,
    lamports_per_sec: u64,
    bypass: bool,
}
#[account]
pub struct PaymentAccount {
    id: u64,
    group_id: u64,
    authority: Pubkey,
    until: u64,
    level: u8,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        seeds = [b"global"],
        bump,
        payer = signer,
        space = 8 + 8 + 8,
    )]
    pub global_data_account: Account<'info, GlobalDataAccount>,
    #[account(
        init,
        seeds = [b"holder"],
        bump,
        payer = signer,
        space = 8,
    )]
    /// CHECK: 
    pub global_holder_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct ModifyGlobalData<'info> {
    #[account(
        constraint = CREATOR.parse::<Pubkey>().unwrap() == signer.key() @ CustomError::InvalidCreator
    )]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"global"],
        bump,
    )]
    pub global_data_account: Account<'info, GlobalDataAccount>,
}
#[derive(Accounts)]
#[instruction(id: u64)]
pub struct CreatePaymentGroup<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        seeds = [b"group", id.to_le_bytes().as_ref()],
        bump,
        payer = signer,
        space = 8 + 8 + 32 + 32 + 8 + 1,
    )]
    pub payment_group_account: Account<'info, PaymentGroupAccount>,
    #[account(
        init,
        seeds = [payment_group_account.key().as_ref()],
        bump,
        payer = signer,
        space = 8,
    )]
    /// CHECK:
    pub payment_group_holder_account: AccountInfo<'info>,
    #[account(
        seeds = [b"global"],
        bump,
    )]
    pub global_data_account: Account<'info, GlobalDataAccount>,
    #[account(
        mut,
        seeds = [b"holder"],
        bump,
    )]
    /// CHECK: 
    pub global_holder_account: AccountInfo<'info>,
    #[account(
        mut,
        constraint = creator.key() == CREATOR.parse::<Pubkey>().unwrap() @ CustomError::InvalidCreator
    )]
    /// CHECK: 
    pub creator: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
#[instruction(id: u64)]
pub struct ModifyPaymentGroup<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"group", id.to_le_bytes().as_ref()],
        bump,
        constraint = payment_group_account.modify_authority == signer.key() @ CustomError::InvalidModifyAuthority
    )]
    pub payment_group_account: Account<'info, PaymentGroupAccount>,
}

#[derive(Accounts)]
#[instruction(group_id: u64, id: u64, level: u8)]
pub struct Pay<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init_if_needed,
        seeds = [b"payment", group_id.to_le_bytes().as_ref(), id.to_le_bytes().as_ref(), level.to_le_bytes().as_ref()],
        bump,
        payer = signer,
        space = 8 + 8 + 8 + 32 + 8 + 1
    )]
    pub payment_account: Account<'info, PaymentAccount>,
    #[account(
        init,
        seeds = [payment_account.key().as_ref()],
        bump,
        payer = signer,
        space = 8,
    )]
    /// CHECK: 
    pub payment_holder_account: AccountInfo<'info>,
    #[account(
        seeds = [b"group", group_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub payment_group_account: Account<'info, PaymentGroupAccount>,
    #[account(
        seeds = [b"global"],
        bump,
    )]
    pub global_data_account: Account<'info, GlobalDataAccount>,
    #[account(
        mut,
        seeds = [b"holder"],
        bump,
    )]
    /// CHECK:
    pub global_holder_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(group_id: u64, id: u64, level: u8)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"payment", group_id.to_le_bytes().as_ref(), id.to_le_bytes().as_ref(), level.to_le_bytes().as_ref()],
        bump,
        constraint = payment_account.authority == signer.key() @ CustomError::InvalidPaymentAuthority
    )]
    pub payment_account: Account<'info, PaymentAccount>,
    #[account(
        mut,
        seeds = [payment_account.key().as_ref()],
        bump,
    )]
    /// CHECK: 
    pub payment_holder_account: AccountInfo<'info>,
    #[account(
        seeds = [b"group", group_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub payment_group_account: Account<'info, PaymentGroupAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(group_id: u64, id: u64, level: u8)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        seeds = [b"payment", group_id.to_le_bytes().as_ref(), id.to_le_bytes().as_ref(), level.to_le_bytes().as_ref()],
        bump,
    )]
    pub payment_account: Account<'info, PaymentAccount>,
    #[account(
        mut,
        seeds = [payment_account.key().as_ref()],
        bump,
    )]
    /// CHECK: 
    pub payment_holder_account: AccountInfo<'info>,
    #[account(
        seeds = [b"group", group_id.to_le_bytes().as_ref()],
        bump,
        constraint = payment_group_account.withdraw_authority == signer.key() @ CustomError::InvalidWithdrawAuthority
    )]
    pub payment_group_account: Account<'info, PaymentGroupAccount>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct WithdrawProgramFunds<'info> {
    #[account(
        mut,
        constraint = CREATOR.parse::<Pubkey>().unwrap() == signer.key() @ CustomError::InvalidCreator
    )]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"holder"],
        bump,
    )]
    /// CHECK: 
    pub global_holder_account: AccountInfo<'info>,
}