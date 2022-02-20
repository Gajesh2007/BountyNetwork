use anchor_lang::prelude::*;
use anchor_lang::solana_program::{clock, program_option::COption, sysvar};
use anchor_spl::token::{self, Mint, Token, TokenAccount};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod permissionless_bounty_network {
    use super::*;
    pub fn initialize_bounty_network(ctx: Context<InitializeBountyNetwork>, nonce: u8) -> ProgramResult {
        let bounty_network = &mut ctx.accounts.bounty_network;
        bounty_network.moderator = ctx.accounts.moderator.key();
        bounty_network.bounty_count = 0;
        bounty_network.nonce = nonce;

        Ok(())
    }

    pub fn create_bounty(ctx: Context<CreateBounty>, title: String, description: String, project: String, reward_amount: u128, deadline: u64, nonce: u8) -> ProgramResult {
        let bounty_network = &mut ctx.accounts.bounty_network;
        let bounty = &mut ctx.accounts.bounty;
        
        bounty.bounty_authority = ctx.accounts.bounty_authority.key();
        bounty.bounty_network = bounty_network.key();
        bounty.bounty_number = bounty_network.bounty_count;
        bounty.title = title;
        bounty.description = description;
        bounty.project = project;
        bounty.submission_count = 0;
        
        if bounty_network.moderator == ctx.accounts.bounty_authority.key() {
            bounty.accepted = true;
        } else {
            bounty.accepted = false;
        }

        bounty.completed = false;
        bounty.reward_amount = reward_amount;
        bounty.reward_token_mint = ctx.accounts.reward_token_mint.key();
        bounty.reward_token_vault = ctx.accounts.reward_token_vault.key();
        bounty.deadline = deadline;
        bounty.nonce = nonce;

        bounty_network.bounty_count += 1;

        Ok(())
    }

    pub fn accept_bounty(ctx: Context<AcceptBounty>, bounty_number: u128) -> ProgramResult {
        
        let bounty = &mut ctx.accounts.bounty;

        bounty.accepted = true;

        Ok(())
    }

    pub fn reject_bounty(ctx: Context<RejectBounty>, bounty_number: u128) -> ProgramResult {
        let bounty_network = &mut ctx.accounts.bounty_network;
        let bounty = &mut ctx.accounts.bounty;

        {
            let seeds = &[bounty_network.to_account_info().key.as_ref(), &[bounty_network.nonce]];
            let pool_signer = &[&seeds[..]];

            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.reward_token_vault.to_account_info(),
                    to: ctx.accounts.project_token_vault.to_account_info(),
                    authority: ctx.accounts.pool_signer.to_account_info(),
                },
                pool_signer,
            );
            token::transfer(cpi_ctx, bounty.reward_amount as u64)?;
        }

        Ok(())
    }

    pub fn submit_work(ctx: Context<SubmitWork>, bounty_number: u128, name: String, link_to_submission: String, email: String, discord: String, twitter: String, anything_else: String, nonce: u8) -> ProgramResult {
        let bounty = &mut ctx.accounts.bounty;
        let submission = &mut ctx.accounts.submission;

        submission.bounty_number = bounty_number;
        submission.name = name;
        submission.link_to_submission = link_to_submission;
        submission.wallet = ctx.accounts.submitter.key();
        submission.email = email;
        submission.discord = discord;
        submission.twitter = twitter;
        submission.anything_else = anything_else;
        
        submission.nonce = nonce;

        bounty.submission_count += 1;

        Ok(())
    }

    pub fn select_winner(ctx: Context<SelectWinner>, bounty_number: u128) -> ProgramResult {
        let bounty_network = &mut ctx.accounts.bounty_network;
        let bounty = &mut ctx.accounts.bounty;
        let submission = &mut ctx.accounts.submission;

        if bounty.completed == true {
            return Err(ErrorCode::BountyAlreadyCompleted.into());
        }

        if bounty.completed == false {
            bounty.completed = true;
        }

        {
            let seeds = &[bounty_network.to_account_info().key.as_ref(), &[bounty_network.nonce]];
            let pool_signer = &[&seeds[..]];

            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.reward_token_vault.to_account_info(),
                    to: ctx.accounts.token_vault.to_account_info(),
                    authority: ctx.accounts.pool_signer.to_account_info(),
                },
                pool_signer,
            );
            token::transfer(cpi_ctx, bounty.reward_amount as u64)?;
        }

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(nonce: u8)]
pub struct InitializeBountyNetwork<'info> {
    #[account(zero)]
    pub bounty_network: Account<'info, BountyNetwork>,

    #[account(mut)]
    pub moderator: Signer<'info>,

    #[account(
        seeds = [
            bounty_network.to_account_info().key.as_ref()
        ],
        bump = nonce,
    )]
    pub pool_signer: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct CreateBounty<'info> {
    #[account(mut)]
    pub bounty_network: Account<'info, BountyNetwork>,

    #[account(
        init,
        payer = bounty_authority,
        seeds = [
            bounty_network.to_account_info().key.as_ref(),
            bounty_network.bounty_count.to_string().as_ref(),
        ],
        bump,
    )]
    pub bounty: Box<Account<'info, Bounty>>,

    #[account(mut)]
    pub bounty_authority: Signer<'info>,

    pub reward_token_mint: Account<'info, Mint>,
    #[account(
        constraint = reward_token_vault.mint == reward_token_mint.key(),
        constraint = reward_token_vault.owner == pool_signer.key()
    )]
    pub reward_token_vault: Account<'info, TokenAccount>,

    #[account(
        seeds = [
            bounty_network.to_account_info().key.as_ref()
        ],
        bump = bounty_network.nonce,
    )]
    pub pool_signer: UncheckedAccount<'info>,

    // Misc
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bounty_number: u128)]
pub struct AcceptBounty<'info> {
    #[account(
        mut,
        has_one = moderator
    )]
    pub bounty_network: Account<'info, BountyNetwork>,

    #[account(
        mut,
        has_one = bounty_network,
        seeds = [
            bounty_network.to_account_info().key.as_ref(),
            bounty_number.to_string().as_ref(),
        ],
        bump = bounty.nonce,
    )]
    pub bounty: Box<Account<'info, Bounty>>,

    #[account(mut)]
    pub moderator: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(bounty_number: u128)]
pub struct RejectBounty<'info> {
    #[account(
        mut,
        has_one = moderator
    )]
    pub bounty_network: Account<'info, BountyNetwork>,

    #[account(
        mut,
        has_one = bounty_network,
        seeds = [
            bounty_network.to_account_info().key.as_ref(),
            bounty_number.to_string().as_ref(),
        ],
        bump = bounty.nonce,
    )]
    pub bounty: Box<Account<'info, Bounty>>,

    #[account(
        constraint = reward_token_vault.mint == bounty.reward_token_mint,
        constraint = reward_token_vault.owner == pool_signer.key()
    )]
    pub reward_token_vault: Account<'info, TokenAccount>,

    #[account(
        constraint = project_token_vault.mint == bounty.reward_token_mint,
        constraint = project_token_vault.owner == bounty.bounty_authority,
    )]
    pub project_token_vault: Account<'info, TokenAccount>,

    #[account(
        seeds = [
            bounty_network.to_account_info().key.as_ref()
        ],
        bump = bounty_network.nonce,
    )]
    pub pool_signer: UncheckedAccount<'info>,

    #[account(mut)]
    pub moderator: Signer<'info>,

    // Misc
    pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
#[instruction(bounty_number: u128)]
pub struct SubmitWork<'info> {
    #[account(mut)]
    pub bounty_network: Account<'info, BountyNetwork>,

    #[account(
        mut,
        has_one = bounty_network,
        seeds = [
            bounty_network.to_account_info().key.as_ref(),
            bounty_number.to_string().as_ref(),
        ],
        bump = bounty.nonce,
    )]
    pub bounty: Box<Account<'info, Bounty>>,

    #[account(
        init,
        payer = submitter,
        seeds = [
            bounty_network.to_account_info().key.as_ref(),
            bounty.to_account_info().key.as_ref(),
            bounty.submission_count.to_string().as_ref(),
        ],
        bump,
    )]
    pub submission: Box<Account<'info, Submission>>,

    #[account(mut)]
    pub submitter: Signer<'info>,

    #[account(
        constraint = token_vault.mint == bounty.reward_token_mint,
        constraint = token_vault.owner == submitter.key()
    )]
    pub token_vault: Account<'info, TokenAccount>,

    #[account(
        seeds = [
            bounty_network.to_account_info().key.as_ref()
        ],
        bump = bounty_network.nonce,
    )]
    pub pool_signer: UncheckedAccount<'info>,

    // Misc
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bounty_number: u128)]
pub struct SelectWinner<'info> {
    #[account(
        mut,
    )]
    pub bounty_network: Account<'info, BountyNetwork>,

    #[account(
        mut,
        has_one = bounty_network,
        has_one = bounty_authority,
        has_one = reward_token_vault,
        seeds = [
            bounty_network.to_account_info().key.as_ref(),
            bounty_number.to_string().as_ref(),
        ],
        bump = bounty.nonce,
    )]
    pub bounty: Box<Account<'info, Bounty>>,

    #[account(
        mut,
        has_one = token_vault,
    )]
    pub submission: Box<Account<'info, Submission>>,

    #[account(mut)]
    pub bounty_authority: Signer<'info>,

    pub reward_token_vault: Account<'info, TokenAccount>,
    pub token_vault: Account<'info, TokenAccount>,

    #[account(
        seeds = [
            bounty_network.to_account_info().key.as_ref()
        ],
        bump = bounty_network.nonce,
    )]
    pub pool_signer: UncheckedAccount<'info>,

    // Misc
    pub token_program: Program<'info, Token>
}

#[account]
#[derive(Default)]
pub struct BountyNetwork {
    pub moderator: Pubkey,
    pub bounty_count: u128,

    pub nonce: u8,
}

#[account]
#[derive(Default)]
pub struct Bounty {
    pub bounty_authority: Pubkey,
    pub bounty_network: Pubkey,
    pub bounty_number: u128,
    pub title: String,

    // description should be written in markdown format
    pub description: String,
    pub project: String,
    pub accepted: bool,
    pub completed: bool,
    
    // reward info
    pub reward_amount: u128,
    pub reward_token_mint: Pubkey,
    pub reward_token_vault: Pubkey,

    pub submission_count: u64,

    pub deadline: u64,

    pub nonce: u8,
}

#[account]
#[derive(Default)]
pub struct Submission {
    pub bounty_number: u128,
    pub name: String,
    pub link_to_submission: String,
    pub wallet: Pubkey,
    pub token_vault: Pubkey,
    pub email: String,
    pub discord: String,
    pub twitter: String,
    pub anything_else: String,

    pub nonce: u8,
}

#[error]
pub enum ErrorCode {
    #[msg("Bounty Already Completed")]
    BountyAlreadyCompleted,
}