import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { GoquantAssignment } from "../target/types/goquant_assignment";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  getAccount,
  getOrCreateAssociatedTokenAccount,
  memoTransferInstructionData,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { prototype } from "mocha";
import { assert } from "chai";

describe("Collateral Vault Management System", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace
    .goquantAssignment as Program<GoquantAssignment>;

  //Test Wallets
  const payer = provider.wallet as anchor.Wallet;
  let user1: Keypair;
  let user2: Keypair;

  //Token Mint (_USDT_)
  let usdtMint: PublicKey;
  let mintAuthority: Keypair;

  //UserTokenAccounts
  let user1TokenAccount: PublicKey;
  let user2TokenAccount: PublicKey;

  //Vault PDA's
  let user1VaultPda: PublicKey;
  let user1Bump: number;
  let user1VaultAta: PublicKey;
  let user1VaultAuthority: PublicKey;
  let user1VaultAuthorityBump: number;

  let user2VaultPda: PublicKey;
  let user2Bump: number;
  let user2VaultAta: PublicKey;
  let user2VaultAuthority: PublicKey;
  let user2VaultAuthorityBump: number;

  const INITIAL_MINT = 10_000 * 1_000_000;
  const DEPOSIT_AMOUNT = 1_000 * 1_000_000;
  const WITHDRAWN_AMOUNT = 500 * 1_000_000;
  const LOCK_AMOUNT = 300 * 1_000_000;

  before(async () => {
    console.log("setting up the testing environment");
    //Create test users
    user1 = Keypair.generate();
    user2 = Keypair.generate();

    //AirdropSol
    const airdropAmount = 5 * anchor.web3.LAMPORTS_PER_SOL;
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(user1.publicKey, airdropAmount)
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(user2.publicKey, airdropAmount)
    );

    console.log("Created Test Users AND airdropped SOL");

    //CREATING MOCK USDT MINT

    mintAuthority = Keypair.generate();
    usdtMint = await createMint(
      provider.connection,
      payer.payer,
      mintAuthority.publicKey,
      null,
      6
    );
    console.log("Created mock USDT mint:", usdtMint.toBase58());

    const user1Account = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer.payer,
      usdtMint,
      user1.publicKey
    );

    user1TokenAccount = user1Account.address;

    const user2Account = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer.payer,
      usdtMint,
      user2.publicKey
    );
    user2TokenAccount = user2Account.address;

    console.log("Created user token accounts");

    await mintTo(
      provider.connection,
      payer.payer,
      usdtMint,
      user1TokenAccount,
      mintAuthority,
      INITIAL_MINT
    );

    await mintTo(
      provider.connection,
      payer.payer,
      usdtMint,
      user2TokenAccount,
      mintAuthority,
      INITIAL_MINT
    );

    console.log("Minted initial USDT to users");

    [user1VaultPda, user1Bump] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), user1.publicKey.toBuffer()],
      program.programId
    );

    [user2VaultPda, user2Bump] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), user2.publicKey.toBuffer()],
      program.programId
    );

    [user1VaultAuthority, user1VaultAuthorityBump] =
      PublicKey.findProgramAddressSync(
        [Buffer.from("vault_authority"), user1VaultPda.toBuffer()],
        program.programId
      );

    [user2VaultAuthority, user2VaultAuthorityBump] =
      PublicKey.findProgramAddressSync(
        [Buffer.from("vault_authority"), user2VaultPda.toBuffer()],
        program.programId
      );

    console.log(" Derived vault PDAs");
    console.log("   User1 Vault:", user1VaultPda.toBase58());
    console.log("   User2 Vault:", user2VaultPda.toBase58());

    console.log("\n Test environment setup complete!\n");
  });

  describe("1. Vault Initialization", () => {
    it("should initialize vault for user 1", async () => {
      user1VaultAta = await anchor.utils.token.associatedAddress({
        mint : usdtMint,
        owner : user1VaultPda
      });
      console.log(user1VaultAta)
      const tx = await program.methods
        .initializeVault()
        .accounts({
          user: user1.publicKey,
          mint: usdtMint,
        })
        .signers([user1])
        .rpc();
      console.log("   Transaction signature:", tx);

      const vaultAccount = await program.account.collateralVault.fetch(
        user1VaultPda
      );

      assert.ok(vaultAccount.owner.equals(user1.publicKey));
      assert.ok(vaultAccount.tokenAccount.equals(user1VaultAta));
      assert.equal(vaultAccount.totalBalance.toNumber(), 0);
      assert.equal(vaultAccount.lockedBalance.toNumber(), 0);
      assert.equal(vaultAccount.availableBalance.toNumber(), 0);
      assert.equal(vaultAccount.totalDeposited.toNumber(), 0);
      assert.equal(vaultAccount.totalWithdrawn.toNumber(), 0);

      const ataAccount = await getAccount(provider.connection, user1VaultAta);
      assert.ok(
        ataAccount.owner.equals(user1VaultPda),
        "vault ATA must be owned by the vault PDA"
      );

      console.log(
        "User1 vault initialized successfully and ATA ownership verified"
      );

      //Authorizing this program ID in the vault AUTHORITY

      await program.methods
        .authorityToAdd(program.programId)
        .accounts({
          admin: user1.publicKey,
        })
        .signers([user1])
        .rpc();

      const va = await program.account.vaultAuthority.fetch(
        user1VaultAuthority
      );
      const contains = va.authorizedPrograms.some((p: PublicKey) => 
        p.equals(program.programId)
      );
      assert.ok(contains, "vault authority should contain the test program id");
      console.log("Program ID added to vault authority for user1");
    });

    it("should initialize vault for user 2", async () => {
      user2VaultAta = await anchor.utils.token.associatedAddress({
        mint : usdtMint,
        owner : user2VaultPda
      });
      
      console.log(user2VaultAta)
      await program.methods
        .initializeVault()
        .accounts({
          user: user2.publicKey,
          mint: usdtMint,
        })
        .signers([user2])
        .rpc();

      const vaultAccount = await program.account.collateralVault.fetch(
        user2VaultPda
      );
      assert.ok(vaultAccount.owner.equals(user2.publicKey));
      assert.ok(vaultAccount.tokenAccount.equals(user2VaultAta));
      assert.equal(vaultAccount.totalBalance.toNumber(), 0);
      assert.equal(vaultAccount.lockedBalance.toNumber(), 0);
      assert.equal(vaultAccount.availableBalance.toNumber(), 0);
      assert.equal(vaultAccount.totalDeposited.toNumber(), 0);
      assert.equal(vaultAccount.totalWithdrawn.toNumber(), 0);

      const ataAccount = await getAccount(provider.connection, user2VaultAta);
      assert.ok(
        ataAccount.owner.equals(user2VaultPda),
        "vault ATA must be owned by the vault PDA"
      );
      console.log(
        "User2 vault initialized successfully and ATA ownership verified"
      );

      await program.methods
        .authorityToAdd(program.programId)
        .accounts({
          admin: user2.publicKey,
        })
        .signers([user2])
        .rpc();

      const va2 = await program.account.vaultAuthority.fetch(
        user2VaultAuthority
      );
      const contains2 = va2.authorizedPrograms.some((p: PublicKey) =>
        p.equals(program.programId)
      );
      assert.ok(
        contains2,
        "vault authority should contain the test program id for user2"
      );

      console.log("User2 vault initialized and program authorized");
    });
    
    it("Should fail to initialize same vault twice", async () => {
      try {
        await program.methods
          .initializeVault()
          .accounts({
            user: user1.publicKey,
            mint: usdtMint,
          })
          .signers([user1])
          .rpc();

        assert.fail("Should have failed to initialize vault twice");
      } catch (error) {
        console.log("Correctly failed to initialize vault twice");
      }
    });
  });
  

  describe("2. Deposit Operations", () => {
    it("should deposit USDT to vault", async () => {
      const balanceBefore = await getAccount(
        provider.connection,
        user1TokenAccount
      );
      await program.methods
        .deposit(new anchor.BN(DEPOSIT_AMOUNT))
        .accounts({
          user: user1.publicKey,
          vaultAta: user1VaultAta,
          userTokenAccount: user1TokenAccount,
        })
        .signers([user1])
        .rpc();

      const balanceAfter = await getAccount(
        provider.connection,
        user1TokenAccount
      );
      assert.equal(
        Number(balanceBefore.amount) - Number(balanceAfter.amount),
        DEPOSIT_AMOUNT
      );
      const vaultTokenBalance = await getAccount(
        provider.connection,
        user1VaultAta
      );
      assert.equal(Number(vaultTokenBalance.amount), DEPOSIT_AMOUNT);

      const vaultAccount = await program.account.collateralVault.fetch(
        user1VaultPda
      );
      assert.equal(vaultAccount.totalBalance.toNumber(), DEPOSIT_AMOUNT);
      assert.equal(vaultAccount.availableBalance.toNumber(), DEPOSIT_AMOUNT);
      assert.equal(vaultAccount.totalDeposited.toNumber(), DEPOSIT_AMOUNT);

      console.log("Deposited", DEPOSIT_AMOUNT / 1_000_000, "USDT");
    });
    it("Should handle multiple deposits correctly", async () => {
      const secondDeposit = 500 * 1_000_000; // 500 USDT

      await program.methods
        .deposit(new anchor.BN(secondDeposit))
        .accounts({
          user: user1.publicKey,
          userTokenAccount: user1TokenAccount,
          vaultAta: user1VaultAta,
        })
        .signers([user1])
        .rpc();

      const vaultAccount = await program.account.collateralVault.fetch(
        user1VaultPda
      );

      const expectedTotal = DEPOSIT_AMOUNT + secondDeposit;
      assert.equal(vaultAccount.totalBalance.toNumber(), expectedTotal);
      assert.equal(vaultAccount.availableBalance.toNumber(), expectedTotal);
      assert.equal(vaultAccount.totalDeposited.toNumber(), expectedTotal);

      console.log("Multiple deposits work correctly");
    });

    it("should fail to deposit zero amount", async () => {
      try {
        await program.methods
          .deposit(new anchor.BN(0))
          .accounts({
            user: user1.publicKey,
            userTokenAccount: user1TokenAccount,
            vaultAta: user1VaultAta,
          })
          .signers([user1])
          .rpc();

        assert.fail("Should have failed to deposit zero");
      } catch (error) {
        assert.ok(error.toString().includes("InvalidAmount"));
        console.log("Correctly rejected zero deposit");
      }
    });
  });
  describe("3. Withdrawal Operations", async () => {
    it("Should withdraw USDT from vault", async () => {
      const vaultBefore = await program.account.collateralVault.fetch(
        user1VaultPda
      );

      const userBalanceBefore = await getAccount(
        provider.connection,
        user1TokenAccount
      );
await 
      await program.methods
        .withdraw(new anchor.BN(WITHDRAWN_AMOUNT))
        .accounts({
          user: user1.publicKey,
          userTokenAccount: user1TokenAccount,
          vaultAta: user1VaultAta,
        })
        .signers([user1])
        .rpc();

      const vaultAfter = await program.account.collateralVault.fetch(
        user1VaultPda
      );

      assert.equal(
        vaultAfter.totalBalance.toNumber(),
        vaultBefore.totalBalance.toNumber() - WITHDRAWN_AMOUNT
      );

      assert.equal(
        vaultAfter.availableBalance.toNumber(),
        vaultBefore.availableBalance.toNumber() - WITHDRAWN_AMOUNT
      );

      assert.equal(vaultAfter.totalWithdrawn.toNumber(), WITHDRAWN_AMOUNT);

      const userBalanceAfter = await getAccount(
        provider.connection,
        user1TokenAccount
      );

      assert.equal(
        Number(userBalanceAfter.amount) - Number(userBalanceBefore.amount),
        WITHDRAWN_AMOUNT
      );
      console.log(" Withdrew", WITHDRAWN_AMOUNT / 1_000_000, "USDT");
    });

    it("should fail to withdraw more than available balance", async () => {
      const vaultAccount = await program.account.collateralVault.fetch(
        user1VaultPda
      );

      const excessiveAmount =
        vaultAccount.availableBalance.toNumber() + 1_000_000;
      try {
        await await program.methods
          .withdraw(new anchor.BN(excessiveAmount))
          .accounts({
            user: user1.publicKey,
            vaultAta: user1VaultAta,
            userTokenAccount: user1TokenAccount,
          })
          .signers([user1])
          .rpc();

        assert.fail("Should have failed to withdraw excessive amount");
      } catch (error) {
        assert.ok(error.toString().includes("InsufficientBalance"));
        console.log("Correctly rejected excessive withdrawal");
      }
    });
    it("Should fail when unauthorized user tries to withdraw", async () => {
      try {
        await await program.methods
          .withdraw(new anchor.BN(100_000))
          .accounts({
            user: user2.publicKey, // Wrong user!
            vaultAta: user1VaultAta,
            userTokenAccount: user2TokenAccount,
          })
          .signers([user2])
          .rpc();

        assert.fail("Should have failed unauthorized withdrawal");
      } catch (error) {
        console.log("Correctly rejected unauthorized withdrawal");
      }
    });
  });

  describe("4. Lock/UnLock Collateral", () => {
    it("should lock collateral", async () => {
      const vaultBefore = await program.account.collateralVault.fetch(
        user1VaultPda
      );

      await program.methods
        .lockCollateral(new anchor.BN(LOCK_AMOUNT))
        .accounts({
          vault: user1VaultPda,
          authorityProgram: program.programId,
        })
        .rpc();

      const vaultAfter = await program.account.collateralVault.fetch(
        user1VaultPda
      );

      assert.equal(
        vaultAfter.lockedBalance.toNumber(),
        vaultBefore.lockedBalance.toNumber() + LOCK_AMOUNT
      );
      assert.equal(
        vaultAfter.availableBalance.toNumber(),
        vaultBefore.availableBalance.toNumber() - LOCK_AMOUNT
      );

      console.log("Locked", LOCK_AMOUNT / 1_000_000, "USDT");
    });

    it("should unlock collateral", async () => {
      const vaultBefore = await program.account.collateralVault.fetch(
        user1VaultPda
      );
      const unlockAmount = LOCK_AMOUNT / 2;
      await program.methods
        .unlockCollateral(new anchor.BN(unlockAmount))
        .accounts({
          vault: user1VaultPda,
          authorityProgram: program.programId,
        }).rpc();

      const vaultAfter = await program.account.collateralVault.fetch(
        user1VaultPda
      );

      assert.equal(
        vaultAfter.lockedBalance.toNumber(),
        vaultBefore.lockedBalance.toNumber() - unlockAmount
      );
      assert.equal(
        vaultAfter.availableBalance.toNumber(),
        vaultBefore.availableBalance.toNumber() + unlockAmount
      );

      console.log("Unlocked", unlockAmount / 1_000_000, "USDT");
    });

    it("Should fail to lock more than available balance", async () => {
      const vaultAccount = await program.account.collateralVault.fetch(
        user1VaultPda
      );
      const excessiveAmount =
        vaultAccount.availableBalance.toNumber() + 1_000_000;

      try {
        await program.methods
          .lockCollateral(new anchor.BN(excessiveAmount))
          .accounts({
            vault: user1VaultPda,
            authorityProgram: program.programId,
          })
          .rpc();

        assert.fail("Should have failed to lock excessive amount");
      } catch (error) {
        assert.ok(error.toString().includes("InsufficientBalance"));
        console.log("Correctly rejected excessive lock");
      }
    });

    it("Should fail to unlock more than locked balance", async () => {
      const vaultAccount = await program.account.collateralVault.fetch(
        user1VaultPda
      );
      const excessiveAmount = vaultAccount.lockedBalance.toNumber() + 1_000_000;

      try {
        await program.methods
          .unlockCollateral(new anchor.BN(excessiveAmount))
          .accounts({
            vault: user1VaultPda,
            authorityProgram: program.programId,
          })
          .rpc();

        assert.fail("Should have failed to unlock excessive amount");
      } catch (error) {
        console.log("Correctly rejected excessive unlock");
      }
    });
  });

  describe("5. Transfer Between Vaults", async () => {
    before(async () => {
      await program.methods
        .deposit(new anchor.BN(DEPOSIT_AMOUNT))
        .accounts({
          user: user2.publicKey,
          userTokenAccount: user2TokenAccount,
          vaultAta: user2VaultAta,
        })
        .signers([user2])
        .rpc();
    });
    it("should transfer collateral between vaults", async () => {
      const transfer_amount = 200 * 1_000_000;

      const vault1Before = await program.account.collateralVault.fetch(
        user1VaultPda
      );
      const vault2Before = await program.account.collateralVault.fetch(
        user2VaultPda
      );

      await program.methods
        .transferCollateral(new anchor.BN(transfer_amount))
        .accounts({
          fromVault: user1VaultPda,
          fromVaultAta: user1VaultAta,
          toVault: user2VaultPda,
          toVaultAta: user2VaultAta,
          authorityProgram: program.programId,
        })
        .rpc();

      const vault1After = await program.account.collateralVault.fetch(
        user1VaultPda
      );
      const vault2After = await program.account.collateralVault.fetch(
        user2VaultPda
      );

      // Verify source vault decreased
      assert.equal(
        vault1After.totalBalance.toNumber(),
        vault1Before.totalBalance.toNumber() - transfer_amount
      );
      assert.equal(
        vault1After.availableBalance.toNumber(),
        vault1Before.availableBalance.toNumber() - transfer_amount
      );

      // Verify destination vault increased
      assert.equal(
        vault2After.totalBalance.toNumber(),
        vault2Before.totalBalance.toNumber() + transfer_amount
      );
      assert.equal(
        vault2After.availableBalance.toNumber(),
        vault2Before.availableBalance.toNumber() + transfer_amount
      );

      console.log(
        " Transferred",
        transfer_amount / 1_000_000,
        "USDT between vaults"
      );
    });

    it("should fail to transfer more than available balance", async () => {
      const vaultAccount = await program.account.collateralVault.fetch(
        user1VaultPda
      );
      const excessive_amount =
        vaultAccount.availableBalance.toNumber() + 1_000_000;
      try {
        await program.methods
          .transferCollateral(new anchor.BN(excessive_amount))
          .accounts({
            fromVault: user1VaultPda,
            toVault: user2VaultPda,
            fromVaultAta: user1VaultAta,
            toVaultAta: user2VaultAta,
            authorityProgram: program.programId,
          })
          .rpc();

        assert.fail("Should have failed to transfer excessive amount");
      } catch (error) {
        assert.ok(error.toString().includes("InsufficientBalance"));
        console.log(" Correctly rejected excessive transfer");
      }
    });

    describe("6. Balance Invariants", async () => {
      it("Should maintain balance invariant: available + locked = total", async () => {
        const vault = await program.account.collateralVault.fetch(
          user1VaultPda
        );

        const calculatedTotal =
          vault.availableBalance.toNumber() + vault.lockedBalance.toNumber();

        assert.equal(
          vault.totalBalance.toNumber(),
          calculatedTotal,
          "Balance invariant broken: available + locked != total"
        );

        console.log(" Balance invariant maintained");
      });

      it("Should match on-chain token balance with vault total_balance", async () => {
        const vault = await program.account.collateralVault.fetch(
          user1VaultPda
        );
        const tokenAccount = await getAccount(
          provider.connection,
          user1VaultAta
        );

        assert.equal(
          vault.totalBalance.toNumber(),
          Number(tokenAccount.amount),
          "Vault total_balance doesn't match actual token balance"
        );

        console.log(" On-chain balances match");
      });
    });
  });

  describe("7. Security Tests", async () => {
    it("should prevent withdrawal when balance is locked", async () => {
      const vault = await program.account.collateralVault.fetch(user1VaultPda);
      const availableBalance = vault.availableBalance.toNumber();

      if (availableBalance > 0) {
        await program.methods
          .lockCollateral(new anchor.BN(availableBalance))
          .accounts({
            vault: user1VaultPda,
            authorityProgram: program.programId,
          })
          .rpc();
      }
      try {
        await program.methods
          .withdraw(new anchor.BN(100_000))
          .accounts({
            user: user1.publicKey,
            vaultAta: user1VaultAta,
            userTokenAccount: user1TokenAccount,
          })
          .signers([user1])
          .rpc();

        assert.fail("Should have failed to withdraw locked funds");
      } catch (error) {
        assert.ok(error.toString().includes("InsufficientBalance"));
        console.log(" Correctly prevented withdrawal of locked funds");
      }
      const vaultAfterLock = await program.account.collateralVault.fetch(
        user1VaultPda
      );
      await program.methods
        .unlockCollateral(new anchor.BN(vaultAfterLock.lockedBalance))
        .accounts({
          vault: user1VaultPda,
          authorityProgram: program.programId,
        })
        .rpc();
    });
  });

  after(async () => {
    console.log("\n Final Vault States:\n");

    const vault1 = await program.account.collateralVault.fetch(user1VaultPda);
    console.log("User1 Vault:");
    console.log(
      "  Total Balance:",
      vault1.totalBalance.toNumber() / 1_000_000,
      "USDT"
    );
    console.log(
      "  Available:",
      vault1.availableBalance.toNumber() / 1_000_000,
      "USDT"
    );
    console.log(
      "  Locked:",
      vault1.lockedBalance.toNumber() / 1_000_000,
      "USDT"
    );
    console.log(
      "  Total Deposited:",
      vault1.totalDeposited.toNumber() / 1_000_000,
      "USDT"
    );
    console.log(
      "  Total Withdrawn:",
      vault1.totalWithdrawn.toNumber() / 1_000_000,
      "USDT"
    );

    const vault2 = await program.account.collateralVault.fetch(user2VaultPda);
    console.log("\nUser2 Vault:");
    console.log(
      "  Total Balance:",
      vault2.totalBalance.toNumber() / 1_000_000,
      "USDT"
    );
    console.log(
      "  Available:",
      vault2.availableBalance.toNumber() / 1_000_000,
      "USDT"
    );
    console.log(
      "  Locked:",
      vault2.lockedBalance.toNumber() / 1_000_000,
      "USDT"
    );

    console.log("\n All tests completed!\n");
  });
});