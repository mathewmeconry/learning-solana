import * as anchor from "@coral-xyz/anchor";

export async function getRandomMultisigPda(
  progamId: anchor.web3.PublicKey
): Promise<[anchor.web3.PublicKey, Buffer]> {
  while (true) {
    const randomName = Buffer.from(
      Math.round(Math.random() * 10000).toString()
    );
    const [multisigPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("multisig"), randomName],
      progamId
    );

    if (!(await isPdaUsed(multisigPda))) {
      return [multisigPda, randomName];
    }
  }
}

export function getProposalPda(
  programId: anchor.web3.PublicKey,
  multisig: anchor.web3.PublicKey,
  id: number
): anchor.web3.PublicKey {
  const [proposalPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("proposal"),
      multisig.toBuffer(),
      new anchor.BN(id).toArrayLike(Buffer, "le", 8),
    ],
    programId
  );
  return proposalPda;
}

export async function isPdaUsed(pda: anchor.web3.PublicKey): Promise<boolean> {
  const pdaData = await anchor.getProvider().connection.getAccountInfo(pda);
  return !!pdaData;
}
