import * as anchor from "@coral-xyz/anchor";
import * as ed from "ed25519";
import { Program } from "@coral-xyz/anchor";
import { OpentgInnerWallet } from "../target/types/opentg_inner_wallet";
import { expect } from "chai";
import { LAMPORTS_PER_SOL } from "@solana/web3.js";

describe("anchor-wallet", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace
    .OpentgInnerWallet as Program<OpentgInnerWallet>;

  let wallet: anchor.web3.Keypair;

  before(() => {
    wallet = anchor.web3.Keypair.generate();
  });

  it("Is initialized!", async () => {
    await program.methods
      .initialize()
      .accounts({
        wallet: wallet.publicKey,
        user: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([wallet])
      .rpc();

    const account = await program.account.wallet.fetch(wallet.publicKey);
    expect(account.isInitialized).to.be.true;
    expect(account.ownerPubkey).to.deep.equal(new Array(32).fill(0));
    expect(account.nonce.toNumber()).to.equal(0);
  });

  // Add more tests for other instructions...
  it("set owner", async () => {
    const ownerPubkey = wallet.publicKey.toBytes().map((x) => Number(x));
    console.log("ownerPubkey", ownerPubkey);

    await program.methods
      .setOwner(ownerPubkey)
      .accounts({
        wallet: wallet.publicKey,
      })
      .signers([program.provider.wallet.payer])
      .rpc();

    const account = await program.account.wallet.fetch(wallet.publicKey);

    console.log("ownerPubkey", ownerPubkey);
    expect(account.isInitialized).to.be.true;
    expect(account.ownerPubkey.join("_")).to.equal(ownerPubkey.join("_"));
    expect(account.nonce.toNumber()).to.equal(0);
  });

  it("deposit", async () => {
    const fromAccount = anchor.web3.Keypair.generate();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        fromAccount.publicKey,
        2 * LAMPORTS_PER_SOL
      ),
      "confirmed"
    );

    console.log("airdrop success");

    const amount = new anchor.BN(0.5 * LAMPORTS_PER_SOL);

    // Create transaction to call the deposit function
    await program.methods
      .deposit(amount)
      .accounts({
        from: fromAccount.publicKey,
        wallet: wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([fromAccount])
      .rpc();

    // Fetch the accounts' balances to verify the deposit
    const fromBalanceAfter = await provider.connection.getBalance(
      fromAccount.publicKey
    );

    const account = await program.account.wallet.fetch(wallet.publicKey);
    const walletBalanceAfter = await provider.connection.getBalance(
      wallet.publicKey
    );

    console.log("from.balance", fromBalanceAfter);
    console.log("account.balance", walletBalanceAfter);
  });

  it("withdraw", async () => {
    const toAccount = anchor.web3.Keypair.generate();

    const amount = new anchor.BN(0.3 * anchor.web3.LAMPORTS_PER_SOL);
    const nonce = new anchor.BN(0); // Initial nonce value, assuming it starts at 0

    // Prepare the message to be signed (amount + nonce)
    const message = Buffer.concat([
      amount.toBuffer("le", 8),
      nonce.toBuffer("le", 8),
    ]);

    const signature = await ed.Sign(message, wallet.secretKey.slice(0, 32));

    const ed25519Instruction =
      anchor.web3.Ed25519Program.createInstructionWithPublicKey({
        publicKey: wallet.publicKey.toBytes(),
        message: message,
        signature: signature,
      });

    await program.methods
      .withdraw(amount, Array.from(signature))
      .accounts({
        wallet: wallet.publicKey,
        recipient: toAccount.publicKey,
        ed25519Program: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .signers([program.provider.wallet.payer])
      .preInstructions([ed25519Instruction]) // Add the additional instruction here
      .rpc();

    const walletBalanceAfter = await provider.connection.getBalance(
      wallet.publicKey
    );

    console.log("account.balance", walletBalanceAfter);
  });

  it("call other contract", async () => {});
});
