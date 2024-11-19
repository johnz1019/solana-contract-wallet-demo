use anchor_lang::prelude::*;
use solana_program::instruction::Instruction;
use solana_program::sysvar::instructions::{load_instruction_at_checked, ID as IX_ID};

pub mod error;
pub mod utils;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod opentg_inner_wallet {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let wallet = &mut ctx.accounts.wallet;
        wallet.is_initialized = true;
        wallet.owner_pubkey = [0; 32];
        wallet.nonce = 0;
        Ok(())
    }

    pub fn set_owner(ctx: Context<SetOwner>, owner_pubkey: [u8; 32]) -> Result<()> {
        let wallet = &mut ctx.accounts.wallet;
        if wallet.is_initialized && wallet.owner_pubkey != [0; 32] {
            return Err(ErrorCode::AccountAlreadyInitialized.into());
        }
        wallet.owner_pubkey = owner_pubkey;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let from = &ctx.accounts.from;
        let to = &ctx.accounts.wallet;

        let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
            &from.key(),
            &to.key(),
            amount,
        );

        anchor_lang::solana_program::program::invoke(
            &transfer_ix,
            &[from.to_account_info(), to.to_account_info()],
        )?;

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64, signature: [u8; 64]) -> Result<()> {
        let wallet = &mut ctx.accounts.wallet;
        let recipient = &ctx.accounts.recipient;

        // Construct the message to be signed (amount + nonce)
        // let message = [&amount.to_le_bytes(), &wallet.nonce.to_le_bytes()].concat();
        let message: Vec<u8> = [&amount.to_le_bytes(), &wallet.nonce.to_le_bytes()]
            .into_iter()
            .flatten()
            .copied()
            .collect();

        // Verify the signature
        verify_signature(
            &wallet.owner_pubkey,
            &message,
            &signature,
            &ctx.accounts.ed25519_program,
        )?;

        if wallet.to_account_info().lamports() < amount {
            return Err(ErrorCode::InsufficientFunds.into());
        }

        // Transfer funds
        **wallet.to_account_info().try_borrow_mut_lamports()? -= amount;
        **recipient.to_account_info().try_borrow_mut_lamports()? += amount;

        // Increment the nonce
        wallet.nonce += 1;

        Ok(())
    }

    pub fn call_external_program(
        ctx: Context<CallExternalProgram>,
        program_id: Pubkey,
        instruction_data: Vec<u8>,
        signature: [u8; 64],
    ) -> Result<()> {
        let wallet = &mut ctx.accounts.wallet;

        // Construct the message to be signed (program_id + instruction_data + nonce)
        let message = [
            program_id.as_ref(),
            &instruction_data,
            &wallet.nonce.to_le_bytes(),
        ]
        .concat();

        // Verify the signature
        verify_signature(
            &wallet.owner_pubkey,
            &message,
            &signature,
            &ctx.accounts.ed25519_program,
        )?;

        // Create the instruction
        let ix = anchor_lang::solana_program::instruction::Instruction {
            program_id,
            accounts: ctx
                .remaining_accounts
                .iter()
                .map(|a| AccountMeta {
                    pubkey: *a.key,
                    is_signer: a.is_signer,
                    is_writable: a.is_writable,
                })
                .collect(),
            data: instruction_data,
        };

        // Invoke the instruction
        anchor_lang::solana_program::program::invoke(&ix, &ctx.remaining_accounts)?;

        // Increment the nonce
        wallet.nonce += 1;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 1 + 32 + 8)]
    pub wallet: Account<'info, Wallet>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetOwner<'info> {
    #[account(mut)]
    pub wallet: Account<'info, Wallet>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub wallet: Account<'info, Wallet>,
    #[account(mut)]
    pub from: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub wallet: Account<'info, Wallet>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub recipient: AccountInfo<'info>,
    /// CHECK: This is the Ed25519 program ID
    #[account(address = IX_ID)]
    pub ed25519_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CallExternalProgram<'info> {
    #[account(mut)]
    pub wallet: Account<'info, Wallet>,
    /// CHECK: This is the Ed25519 program ID
    #[account(address = IX_ID)]
    pub ed25519_program: AccountInfo<'info>,
}

#[account]
pub struct Wallet {
    pub is_initialized: bool,
    pub owner_pubkey: [u8; 32],
    pub nonce: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Account is already initialized")]
    AccountAlreadyInitialized,
    #[msg("Invalid signature")]
    InvalidSignature,
    #[msg("Insufficient funds")]
    InsufficientFunds,
}

pub fn verify_signature(
    pubkey: &[u8; 32],
    message: &[u8],
    signature: &[u8; 64],
    ed25519_program: &AccountInfo,
) -> Result<()> {
    let ix: Instruction = load_instruction_at_checked(0, ed25519_program)?;

    utils::verify_ed25519_ix(&ix, pubkey, message, signature)?;

    Ok(())
}
