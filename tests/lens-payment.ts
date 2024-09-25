import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { assert } from "chai";
import { LensPayment } from "../target/types/lens_payment";
import { Keypair, PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { BN } from "bn.js";

describe("lens-payment", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const wallet = provider.wallet as anchor.Wallet;
  const CREATOR = new PublicKey("Ddi1GaugnX9yQz1WwK1b12m4o23rK1krZQMcnt2aNW97");
  const program = anchor.workspace.LensPayment as Program<LensPayment>;
  async function initializeMain() {
    await program.methods.initialize().accounts({
      signer: wallet.publicKey
    }).rpc();
    await program.methods.createPaymentGroup(new BN(26), wallet.publicKey, new BN(10), true).accounts({
      signer: wallet.publicKey,
      creator: CREATOR,
    }).rpc();
  }
  if (provider.connection.rpcEndpoint.includes("devnet") || provider.connection.rpcEndpoint.includes("mainnet")) {
    initializeMain()
  } else {
  it("initializes", async () => {
    await program.methods.initialize().accounts({
      signer: wallet.publicKey
    }).rpc();
  })
  const withdraw = Keypair.generate();
  it("creates payment group account", async () => {
    await provider.connection.requestAirdrop(withdraw.publicKey, LAMPORTS_PER_SOL);
    await program.methods.createPaymentGroup(new BN(100), withdraw.publicKey, new anchor.BN(10), false).accounts({
      signer: wallet.publicKey,
      creator: CREATOR,
    }).rpc(); 
  });
  it("fails to create with same id", async () => {
    try {
      await program.methods.createPaymentGroup(new BN(100), withdraw.publicKey, new anchor.BN(10), false).accounts({
        signer: wallet.publicKey,
        creator: CREATOR,
      }).rpc();
      assert(false, "Failed to throw error");
    } catch (e){

    }
  })
  it("pays", async () => {
    const time = Math.floor(Date.now() / 1000);
    await new Promise(resolve => setTimeout(resolve, 1000));
    await program.methods.pay(new BN(100), new BN(100), 1, new BN(100)).accounts({
      signer: wallet.publicKey,
    }).rpc();    
    const [paymentAccountAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("payment"), new BN(100).toArrayLike(Buffer, "le", 8), new BN(100).toArrayLike(Buffer, "le", 8), new BN(1).toArrayLike(Buffer, "le", 1)],
      program.programId,
    );
    const paymentAccount = await program.account.paymentAccount.fetch(paymentAccountAddress);
    assert(paymentAccount.groupId.eq(new BN(100)), "group id not set");
    assert(paymentAccount.id.eq(new BN(100)), "id not set");
    console.log(paymentAccount.until.toNumber(), new BN(time + 100).toNumber());
    assert(paymentAccount.until.gte(new BN(time + 100)), "Until not set correctly")
    assert(paymentAccount.level === 1, "Level not set");
  });
  it("cancels", async () =>{
    await new Promise((resolve) => setTimeout(resolve, 10 * 1000));
    const [paymentAccountAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("payment"), new BN(100).toArrayLike(Buffer, "le", 8), new BN(100).toArrayLike(Buffer, "le", 8), new BN(1).toArrayLike(Buffer, "le", 1)],
      program.programId
    );
    const paymentAccountBefore = await program.account.paymentAccount.fetch(paymentAccountAddress);
    await program.methods.cancel(new BN(100), new BN(100), 1, new BN(10)).accounts({
      signer: wallet.publicKey,
    }).rpc();
    const paymentAccountAfter = await program.account.paymentAccount.fetch(paymentAccountAddress);
    assert(paymentAccountBefore.until.gt(paymentAccountAfter.until), "did not reduce until");
  })
  it("fails to cancel too much", async () => {
    try {
      await program.methods.cancel(new BN(100), new BN(100), 1, new BN(50)).accounts({
        signer: wallet.publicKey
      }).rpc();
      assert(false, "Cancelled too much")
    } catch (e) {
    }
  });
  it("withdraws", async () => {
    const balanceBefore = await provider.connection.getBalance(withdraw.publicKey);
    await program.methods.withdraw(new BN(100), new BN(100), 1).accounts({
      signer: withdraw.publicKey,
    }).signers([withdraw]).rpc();
    const balanceAfter = await provider.connection.getBalance(withdraw.publicKey);
    await new Promise((resolve) => setTimeout(resolve, 1000));
    assert(balanceAfter > balanceBefore, "Balance did not change");
  })
  it("withdraws program funds", async () => {
    const balanceBefore = await provider.connection.getBalance(wallet.publicKey);
    await program.methods.withdrawProgramFunds().accounts({
      signer: wallet.publicKey
    }).rpc();
    const balanceAfter = await provider.connection.getBalance(wallet.publicKey);
    await new Promise((resolve) => setTimeout(resolve, 1000));
    // assert(balanceAfter > balanceBefore, "Balance did not change");
  })
}
});
