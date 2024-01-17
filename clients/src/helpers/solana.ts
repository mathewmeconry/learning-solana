import {
  AccountMeta,
  Connection,
  PublicKey,
  TransactionInstruction,
  clusterApiUrl,
} from "@solana/web3.js";

export function connect(target: string): Connection {
  const connection = new Connection(clusterApiUrl("devnet"));
  return connection;
}

export function getProgramTransaction(
  programId: PublicKey,
  data?: Buffer,
  additionalKeys: AccountMeta[] = []
): TransactionInstruction {
  const txi = new TransactionInstruction({
    keys: [
      {
        isSigner: false,
        isWritable: true,
        pubkey: programId,
      },
      ...additionalKeys,
    ],
    programId,
    data,
  });

  return txi;
}
