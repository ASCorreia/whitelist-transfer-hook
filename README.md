# Whitelist Transfer Hook

This example demonstrates how to implement a transfer hook using the SPL Token 2022 Transfer Hook interface to enforce whitelist restrictions on token transfers.

In this example, only whitelisted addresses will be able to transfer tokens that have this transfer hook enabled, providing fine-grained access control over token movements.

---

## Let's walk through the architecture:

For this program, we will have 1 main state account:

- A Whitelist account

A Whitelist account consists of:

```rust
#[account]
pub struct Whitelist {
    pub address: Vec<Pubkey>,
    pub bump: u8,
}
```

### In this state account, we will store:

- address: A dynamic vector containing all the public keys that are authorized to transfer tokens.
- bump: The bump seed used to derive the whitelist PDA.

This account uses a dynamic vector structure that can grow and shrink as addresses are added or removed from the whitelist.

---

### The admin will be able to create new Whitelist accounts. For that, we create the following context:

```rust
#[derive(Accounts)]
pub struct InitializeWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + 4 + std::mem::size_of::<Pubkey>(),
        seeds = [b"whitelist"],
        bump
    )]
    pub whitelist: Account<'info, Whitelist>,
    pub system_program: Program<'info, System>,
}
```

Let´s have a closer look at the accounts that we are passing in this context:

- admin: Will be the person creating the whitelist account. He will be a signer of the transaction, and we mark his account as mutable as we will be deducting lamports from this account.

- whitelist: Will be the state account that we will initialize and the admin will be paying for the initialization of the account. We derive the Whitelist PDA from the byte representation of the word "whitelist".

- system_program: Program responsible for the initialization of any new account.

### We then implement some functionality for our InitializeWhitelist context:

```rust
impl<'info> InitializeWhitelist<'info> {
    pub fn initialize_whitelist(&mut self, bumps: InitializeWhitelistBumps) -> Result<()> {
        // Initialize the whitelist with an empty address vector
        self.whitelist.set_inner(Whitelist { 
            address: vec![],
            bump: bumps.whitelist,
        });

        Ok(())
    }
}
```

In here, we set the initial data of our Whitelist account with an empty vector of addresses and store the bumps.

---

### The admin will be able to manage whitelist operations (add/remove addresses):

```rust
#[derive(Accounts)]
pub struct WhitelistOperations<'info> {
    #[account(
        mut,
    )]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"whitelist"],
        bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
    pub system_program: Program<'info, System>,
}
```

In this context, we are passing all the accounts needed to manage the whitelist:

- admin: The address of the platform admin. He will be a signer of the transaction, and we mark his account as mutable as he may need to pay for account reallocation fees.

- whitelist: The state account that we will modify. We derive the Whitelist PDA from the byte representation of the word "whitelist".

- system_program: Program responsible for account reallocation and CPI transfers when the whitelist size changes.

### We then implement some functionality for our WhitelistOperations context:

```rust
impl<'info> WhitelistOperations<'info> {
    pub fn add_to_whitelist(&mut self, address: Pubkey) -> Result<()> {
        if !self.whitelist.address.contains(&address) {
            self.realloc_whitelist(true)?;
            self.whitelist.address.push(address);
        }
        Ok(())
    }

    pub fn remove_from_whitelist(&mut self, address: Pubkey) -> Result<()> {
        if let Some(pos) = self.whitelist.address.iter().position(|&x| x == address) {
            self.whitelist.address.remove(pos);
            self.realloc_whitelist(false)?;
        }
        Ok(())
    }

    pub fn realloc_whitelist(&self, is_adding: bool) -> Result<()> {
        // Get the account info for the whitelist
        let account_info = self.whitelist.to_account_info();

        if is_adding {  // Adding to whitelist
            let new_account_size = account_info.data_len() + std::mem::size_of::<Whitelist>();
            // Calculate rent required for the new account size
            let lamports_required = (Rent::get()?).minimum_balance(new_account_size);
            // Determine additional rent required
            let rent_diff = lamports_required - account_info.lamports();

            // Perform transfer of additional rent
            let cpi_program = self.system_program.to_account_info();
            let cpi_accounts = system_program::Transfer{
                from: self.admin.to_account_info(), 
                to: account_info.clone(),
            };
            let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
            system_program::transfer(cpi_context,rent_diff)?;

            // Reallocate the account
            account_info.resize(new_account_size)?;
            msg!("Account Size Updated: {}", account_info.data_len());

        } else {        // Removing from whitelist
            let new_account_size = account_info.data_len() - std::mem::size_of::<Whitelist>();
            // Calculate rent required for the new account size
            let lamports_required = (Rent::get()?).minimum_balance(new_account_size);
            // Determine additional rent to be refunded
            let rent_diff = account_info.lamports() - lamports_required;

            // Reallocate the account
            account_info.resize(new_account_size)?;
            msg!("Account Size Downgraded: {}", account_info.data_len());

            // Perform transfer to refund additional rent
            **self.admin.to_account_info().try_borrow_mut_lamports()? += rent_diff;
            **self.whitelist.to_account_info().try_borrow_mut_lamports()? -= rent_diff;
        }

        Ok(())
    }
}
```
In here, we implement the logic to dynamically add and remove addresses from the whitelist, adding or deducing lamports from the whitelist account to take in consideration the new size.

- When adding a new address to the whitelist, we first calculate and transfer additional rent from the admin to the whitelist account via CPI, then resize the account to accommodate the new data, and lastly we add the new address to the vector.

- When removing an address from the whitelist, we first find the address in the vector and remove it, then we resize the account to a smaller size and refund the excess rent back to the admin using direct lamport manipulation.

---

### The system will need to initialize extra account metadata for the transfer hook:

```rust
#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        init,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        space = ExtraAccountMetaList::size_of(
            InitializeExtraAccountMetaList::extra_account_metas()?.len()
        )?,
        payer = payer
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
}
```

In this context, we are passing all the accounts needed to set up the transfer hook metadata:

- payer: The address paying for the initialization. He will be a signer of the transaction, and we mark his account as mutable as we will be deducting lamports from this account.

- extra_account_meta_list: The account that will store the extra metadata required for the transfer hook. This account is derived from the byte representation of "extra-account-metas" and the mint's public key.

- mint: The token mint that will have the transfer hook enabled.

- system_program: Program responsible for the initialization of any new account.

### We then implement some functionality for our InitializeExtraAccountMetaList context:

```rust
impl<'info> InitializeExtraAccountMetaList<'info> {
    pub fn extra_account_metas() -> Result<Vec<ExtraAccountMeta>> {
        // Derive the whitelist PDA using our program ID
        let (whitelist_pda, _bump) = Pubkey::find_program_address(
            &[b"whitelist"],
            &ID
        );

        Ok(
            vec![
                ExtraAccountMeta::new_with_pubkey(&whitelist_pda.to_bytes().into(), false, false).unwrap(),
            ]
        )
    }
}
```

In here, we define the extra accounts that will be required during transfer hook execution. We pre-compute the whitelist PDA using `find_program_address` and include it using `new_with_pubkey`. This ensures the whitelist account is included in every transfer validation.

---

### The transfer hook will validate every token transfer:

```rust
#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(
        token::mint = mint, 
        token::authority = owner,
    )]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        token::mint = mint,
    )]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()], 
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(
        seeds = [b"whitelist"], 
        bump = whitelist.bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
}
```

In this context, we are passing all the accounts needed for transfer validation:

- source_token: The token account from which tokens are being transferred. We validate that it belongs to the correct mint and is owned by the owner.

- mint: The token mint being transferred.

- destination_token: The token account to which tokens are being transferred. We validate that it belongs to the correct mint.

- owner: The owner of the source token account. This can be a system account or a PDA owned by another program.

- extra_account_meta_list: The metadata account that contains information about extra accounts required for this transfer hook.

- whitelist: The whitelist account that contains the authorized addresses.

### We then implement some functionality for our TransferHook context:

```rust
impl<'info> TransferHook<'info> {
    /// This function is called when the transfer hook is executed.
    pub fn transfer_hook(&mut self, _amount: u64) -> Result<()> {
        // Fail this instruction if it is not called from within a transfer hook
        self.check_is_transferring()?;

        msg!("Source token owner: {}", self.source_token.owner);
        msg!("Destination token owner: {}", self.destination_token.owner);

        if self.whitelist.address.contains(&self.source_token.owner) {
            msg!("Transfer allowed: The address is whitelisted");
        } else {
            panic!("TransferHook: Address is not whitelisted");
        }

        Ok(())
    }

    /// Checks if the transfer hook is being executed during a transfer operation.
    fn check_is_transferring(&mut self) -> Result<()> {
        // Ensure that the source token account has the transfer hook extension enabled

        // Get the account info of the source token account
        let source_token_info = self.source_token.to_account_info();
        // Borrow the account data mutably
        let mut account_data_ref: RefMut<&mut [u8]> = source_token_info.try_borrow_mut_data()?;

        // Unpack the account data as a PodStateWithExtensionsMut
        // This will allow us to access the extensions of the token account
        // We use PodStateWithExtensionsMut because TokenAccount is a POD (Plain Old Data) type
        let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
        // Get the TransferHookAccount extension
        // Search for the TransferHookAccount extension in the token account
        // The returning struct has a `transferring` field that indicates if the account is in the middle of a transfer operation
        let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

        // Check if the account is in the middle of a transfer operation
        if !bool::from(account_extension.transferring) {
            panic!("TransferHook: Not transferring");
        }

        Ok(())
    }
}
```

In this implementation, we first verify that the hook is being called during an actual transfer operation by checking the transfer hook account extension. Then we log the source and destination token owners for debugging purposes. Finally, we validate that the owner of the source token account is present in our whitelist. If the address is whitelisted, the transfer is allowed; otherwise, it fails with a panic, preventing unauthorized token movements.

The transfer hook integrates seamlessly with the SPL Token 2022 transfer process, automatically validating every transfer attempt against the maintained whitelist without requiring additional user intervention.

---

This whitelist transfer hook provides a robust access control mechanism for Token 2022 mints, ensuring that only pre-approved addresses can transfer tokens while maintaining the standard token interface that users and applications expect. 