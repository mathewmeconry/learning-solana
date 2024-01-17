import { config } from "dotenv";
config();
import { Keypair, PublicKey, Transaction, sendAndConfirmTransaction } from "@solana/web3.js";
import { connect, getProgramTransaction } from "../helpers/solana";

async function main() {
  const keypair = Keypair.fromSecretKey(
    Buffer.from(process.env.PRIVATE_KEY as string, "hex")
  );

  const txi = getProgramTransaction(
    new PublicKey("45JmaN1qdykWFZ4r7z1hJbQ8HQGHpzrEdWTztpE6Mm7t")
  );

  const tx = new Transaction().add(txi)
  const sig = await sendAndConfirmTransaction(connect("devnet"), tx, [keypair])
  console.log(sig)
}

main();
