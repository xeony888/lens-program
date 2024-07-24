import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { assert } from "chai";
import { LensPayment } from "../target/types/lens_payment";
import { PublicKey } from "@solana/web3.js";
import { publicKey } from "@coral-xyz/anchor/dist/cjs/utils";

describe("lens-payment", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const wallet = provider.wallet as anchor.Wallet;

  const program = anchor.workspace.LensPayment as Program<LensPayment>;
  it("initializes", async () => {
    await program.methods.initialize().accounts({
      signer: wallet.publicKey
    }).rpc();
  })
  it("initializes payment account", async () => {
    await program.methods.initializePaymentAccount("test", 1).accounts({
      signer: wallet.publicKey,
    }).rpc();
  })
  it("pays", async () => {
    const bn = new anchor.BN(1)
    const [address] = PublicKey.findProgramAddressSync(
      [Buffer.from("holder"), Buffer.from("test"), bn.toArrayLike(Buffer, "le", 1)],
      program.programId,
    );
    const [address2] = PublicKey.findProgramAddressSync(
      [Buffer.from("payment"), Buffer.from("test"), bn.toArrayLike(Buffer, "le", 1)],
      program.programId,
    );
    const balanceBefore = await provider.connection.getBalance(address);
    await program.methods.pay("test", 1, new anchor.BN(100)).accounts({
      signer: wallet.publicKey,
    }).rpc();
    const balanceAfter = await provider.connection.getBalance(address);
    assert(balanceAfter === balanceBefore + 100 * 100);
  });
  it("fails to cancel too much", async () => {
    try {
      await program.methods.cancel("test", 1, new anchor.BN(101)).accounts({
        signer: wallet.publicKey
      }).rpc();
      assert(false);
    } catch (e) {

    }
  })
  it("cancels", async () => {
    const bn = new anchor.BN(1);
    const [address] = PublicKey.findProgramAddressSync(
      [Buffer.from("holder"), Buffer.from("test"), bn.toArrayLike(Buffer, "le", 1)],
      program.programId,
    );
    const [account] = PublicKey.findProgramAddressSync(
      [Buffer.from("payment"), Buffer.from("test"), bn.toArrayLike(Buffer, "le", 1)],
      program.programId
    );
    const paymentAccountBefore = await program.account.paymentAccount.fetch(account);
    const balanceBefore = await provider.connection.getBalance(address);
    await program.methods.cancel("test", 1, new anchor.BN(90)).accounts({
      signer: wallet.publicKey
    }).rpc();
    const balanceAfter = await provider.connection.getBalance(address);
    assert(balanceAfter === balanceBefore - 90 * 100);
    const paymentAccountAfter = await program.account.paymentAccount.fetch(account);
    assert(paymentAccountBefore.until.gte(paymentAccountAfter.until));
  });
  it("withdraws", async () => {
    // remember to add back checks
    const [account] = PublicKey.findProgramAddressSync(
      [Buffer.from("holder"), Buffer.from("test"), new anchor.BN(1).toArrayLike(Buffer, "le", 1)],
      program.programId,
    );
    const balanceBefore = await provider.connection.getBalance(account);
    await program.methods.withdraw("test", 1).accounts({
      signer: wallet.publicKey
    }).rpc();
    const balanceAfter = await provider.connection.getBalance(account);

    assert(balanceBefore > balanceAfter, "Did not lose sol");
    
  })
});
