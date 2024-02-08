import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorMultisig } from "../target/types/anchor_multisig";
import { assert, expect } from "chai";
import { getProposalPda, getRandomMultisigPda } from "./helpers";

describe("anchor-multisig", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AnchorMultisig as Program<AnchorMultisig>;
  const member1 = provider.wallet as anchor.Wallet;
  const member2 = anchor.web3.Keypair.generate();
  const notAMember = anchor.web3.Keypair.generate();
  let multisigPda: anchor.web3.PublicKey;
  let multisigName: Buffer;

  before(async () => {
    await provider.connection.requestAirdrop(member2.publicKey, 1000000000);
    await provider.connection.requestAirdrop(notAMember.publicKey, 1000000000);
  });

  beforeEach(async () => {
    [multisigPda, multisigName] = await getRandomMultisigPda(program.programId);

    await program.methods
      .create(
        Buffer.from(multisigName),
        [member1.publicKey, member2.publicKey],
        new anchor.BN(1)
      )
      .accounts({
        payer: member1.publicKey,
        multisig: multisigPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await provider.connection.requestAirdrop(multisigPda, 1000000000);
  });

  it("create new multisig", async () => {
    const multisigData = await program.account.multisig.fetch(multisigPda);
    expect(multisigData.name.toString()).eq(multisigName.toString());
    expect(multisigData.members.length).eq(2);
    expect(multisigData.members[0].toBase58()).eq(member1.publicKey.toBase58());
    expect(multisigData.members[1].toBase58()).eq(member2.publicKey.toBase58());
    expect(multisigData.threshold.toNumber()).eq(1);
  });

  it("should not allow duplicated members", async () => {
    [multisigPda, multisigName] = await getRandomMultisigPda(program.programId);

    try {
      await program.methods
        .create(
          Buffer.from(multisigName),
          [member1.publicKey, member1.publicKey],
          new anchor.BN(1)
        )
        .accounts({
          payer: member1.publicKey,
          multisig: multisigPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
    } catch (e) {
      expect(e.error.errorMessage).eq("Already member");
      return;
    }

    assert(false, "should have failed");
  });

  it("should not allow removeMember", async () => {
    try {
      await program.methods
        .removeMember(member1.publicKey)
        .accounts({
          multisig: multisigPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
    } catch (e) {
      expect(e).to.be.instanceOf(Error);
      expect(e.message).to.be.eq(`Signature verification failed.
Missing signature for public key [\`${multisigPda}\`].`);
      return;
    }
    assert(false, "should have failed");
  });

  it("should not allow addMember", async () => {
    try {
      await program.methods
        .addMember(notAMember.publicKey)
        .accounts({
          multisig: multisigPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
    } catch (e) {
      expect(e).to.be.instanceOf(Error);
      expect(e.message).to.be.eq(`Signature verification failed.
Missing signature for public key [\`${multisigPda}\`].`);
      return;
    }
    assert(false, "should have failed");
  });

  it("should not allow updateThreshold", async () => {
    try {
      await program.methods
        .updateThreshold(new anchor.BN(2))
        .accounts({
          multisig: multisigPda,
        })
        .rpc();
    } catch (e) {
      expect(e).to.be.instanceOf(Error);
      expect(e.message).to.be.eq(`Signature verification failed.
Missing signature for public key [\`${multisigPda}\`].`);
      return;
    }
    assert(false, "should have failed");
  });

  it("create a new proposal", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    await program.methods
      .createProposal(new anchor.BN(proposalId), [])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const proposalData = await program.account.proposal.fetch(proposalPda);
    expect(proposalData.id.toNumber()).eq(proposalId);
    expect(proposalData.actions.length).eq(0);
    expect(proposalData.executed).eq(false);
    expect(proposalData.approvers.length).eq(0);
  });

  it("should not allow proposal creation for non member", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    try {
      await program.methods
        .createProposal(new anchor.BN(proposalId), [])
        .accounts({
          multisig: multisigPda,
          proposal: proposalPda,
          signer: notAMember.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([notAMember])
        .rpc();
    } catch (e) {
      expect(e.error.errorMessage).eq("Not a member of this multisig");
      return;
    }

    assert(false, "should have failed");
  });

  it("approve a proposal", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    await program.methods
      .createProposal(new anchor.BN(proposalId), [])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .approveProposal()
      .accounts({
        signer: member1.publicKey,
        multisig: multisigPda,
        proposal: proposalPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const proposalData = await program.account.proposal.fetch(proposalPda);
    expect(proposalData.approvers.length).eq(1);
    expect(proposalData.approvers[0].toBase58()).eq(
      member1.publicKey.toBase58()
    );
  });

  it("should not allow approval for non member", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    await program.methods
      .createProposal(new anchor.BN(proposalId), [])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    try {
      await program.methods
        .approveProposal()
        .accounts({
          signer: notAMember.publicKey,
          multisig: multisigPda,
          proposal: proposalPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([notAMember])
        .rpc();
    } catch (e) {
      expect(e.error.errorMessage).eq("Not a member of this multisig");
      return;
    }
    assert(false, "should have failed");
  });

  it("should not allow execution when threshold is not reached", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    await program.methods
      .createProposal(new anchor.BN(proposalId), [])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    try {
      await program.methods
        .executeProposal()
        .accounts({
          multisig: multisigPda,
          proposal: proposalPda,
        })
        .rpc();
    } catch (e) {
      expect(e.msg).eq("Not enough approvals");
      return;
    }

    assert(false, "should have failed");
  });

  it("should execute the proposal", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    await program.methods
      .createProposal(new anchor.BN(proposalId), [])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .approveProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .executeProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
      })
      .rpc();

    const proposalData = await program.account.proposal.fetch(proposalPda);
    expect(proposalData.executed).eq(true);
  });

  it("should not execute the proposal twice", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    await program.methods
      .createProposal(new anchor.BN(proposalId), [])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .approveProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .executeProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
      })
      .rpc();

    try {
      await program.methods
        .executeProposal()
        .accounts({
          multisig: multisigPda,
          proposal: proposalPda,
        })
        .rpc();
    } catch (e) {
      expect(e.msg).eq("Already executed");
      return;
    }
    assert(false, "should have failed");
  });

  it("should add a new member", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    const addMemberInstruction = await program.methods
      .addMember(notAMember.publicKey)
      .instruction();

    await program.methods
      .createProposal(new anchor.BN(proposalId), [
        {
          programId: addMemberInstruction.programId,
          data: addMemberInstruction.data,
          accounts: [
            {
              pubkey: multisigPda,
              isSigner: true,
              isWritable: true,
            },
            {
              pubkey: anchor.web3.SystemProgram.programId,
              isSigner: false,
              isWritable: false,
            },
            {
              pubkey: program.programId,
              isSigner: false,
              isWritable: false,
            },
          ],
        },
      ])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .approveProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .executeProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
      })
      .remainingAccounts([
        {
          pubkey: multisigPda,
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: anchor.web3.SystemProgram.programId,
          isSigner: false,
          isWritable: false,
        },
        {
          pubkey: program.programId,
          isSigner: false,
          isWritable: false,
        },
      ])
      .rpc();

    const multisigData = await program.account.multisig.fetch(multisigPda);
    expect(multisigData.members.length).eq(3);
    expect(multisigData.members[0].toBase58()).eq(member1.publicKey.toBase58());
    expect(multisigData.members[1].toBase58()).eq(member2.publicKey.toBase58());
    expect(multisigData.members[2].toBase58()).eq(
      notAMember.publicKey.toBase58()
    );
  });

  it("should not add a member twice", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    const addMemberInstruction = await program.methods
      .addMember(member1.publicKey)
      .instruction();

    await program.methods
      .createProposal(new anchor.BN(proposalId), [
        {
          programId: addMemberInstruction.programId,
          data: addMemberInstruction.data,
          accounts: [
            {
              pubkey: multisigPda,
              isSigner: true,
              isWritable: true,
            },
            {
              pubkey: anchor.web3.SystemProgram.programId,
              isSigner: false,
              isWritable: false,
            },
            {
              pubkey: program.programId,
              isSigner: false,
              isWritable: false,
            },
          ],
        },
      ])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .approveProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    try {
      await program.methods
        .executeProposal()
        .accounts({
          multisig: multisigPda,
          proposal: proposalPda,
        })
        .remainingAccounts([
          {
            pubkey: multisigPda,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: program.programId,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
    } catch (e) {
      expect(e.msg).eq("Already member");
      return;
    }
    assert(false, "should have failed");
  });

  it("should update the threshold", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    const updateThresholdInstruction = await program.methods
      .updateThreshold(new anchor.BN(2))
      .instruction();

    await program.methods
      .createProposal(new anchor.BN(proposalId), [
        {
          programId: updateThresholdInstruction.programId,
          data: updateThresholdInstruction.data,
          accounts: [
            {
              pubkey: multisigPda,
              isSigner: true,
              isWritable: true,
            },
            {
              pubkey: program.programId,
              isSigner: false,
              isWritable: false,
            },
          ],
        },
      ])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .approveProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .executeProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
      })
      .remainingAccounts([
        {
          pubkey: multisigPda,
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: program.programId,
          isSigner: false,
          isWritable: false,
        },
      ])
      .rpc();

    const multisigData = await program.account.multisig.fetch(multisigPda);
    expect(multisigData.threshold.toNumber()).eq(2);
  });

  it("should not allow high threshold", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    const updateThresholdInstruction = await program.methods
      .updateThreshold(new anchor.BN(5))
      .instruction();

    await program.methods
      .createProposal(new anchor.BN(proposalId), [
        {
          programId: updateThresholdInstruction.programId,
          data: updateThresholdInstruction.data,
          accounts: [
            {
              pubkey: multisigPda,
              isSigner: true,
              isWritable: true,
            },
            {
              pubkey: program.programId,
              isSigner: false,
              isWritable: false,
            },
          ],
        },
      ])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .approveProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    try {
      await program.methods
        .executeProposal()
        .accounts({
          multisig: multisigPda,
          proposal: proposalPda,
        })
        .remainingAccounts([
          {
            pubkey: multisigPda,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: program.programId,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
    } catch (e) {
      expect(e.msg).eq("Threshold too high");
      return;
    }

    assert(false, "should have failed");
  });

  it("should not allow low threshold", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    const updateThresholdInstruction = await program.methods
      .updateThreshold(new anchor.BN(0))
      .instruction();

    await program.methods
      .createProposal(new anchor.BN(proposalId), [
        {
          programId: updateThresholdInstruction.programId,
          data: updateThresholdInstruction.data,
          accounts: [
            {
              pubkey: multisigPda,
              isSigner: true,
              isWritable: true,
            },
            {
              pubkey: program.programId,
              isSigner: false,
              isWritable: false,
            },
          ],
        },
      ])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .approveProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    try {
      await program.methods
        .executeProposal()
        .accounts({
          multisig: multisigPda,
          proposal: proposalPda,
        })
        .remainingAccounts([
          {
            pubkey: multisigPda,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: program.programId,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
    } catch (e) {
      expect(e.msg).eq("Threshold too low");
      return;
    }

    assert(false, "should have failed");
  });

  it("should add a remove member", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    const removeMemberInstruction = await program.methods
      .removeMember(member2.publicKey)
      .instruction();

    await program.methods
      .createProposal(new anchor.BN(proposalId), [
        {
          programId: removeMemberInstruction.programId,
          data: removeMemberInstruction.data,
          accounts: [
            {
              pubkey: multisigPda,
              isSigner: true,
              isWritable: true,
            },
            {
              pubkey: anchor.web3.SystemProgram.programId,
              isSigner: false,
              isWritable: false,
            },
            {
              pubkey: program.programId,
              isSigner: false,
              isWritable: false,
            },
          ],
        },
      ])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .approveProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .executeProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
      })
      .remainingAccounts([
        {
          pubkey: multisigPda,
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: anchor.web3.SystemProgram.programId,
          isSigner: false,
          isWritable: false,
        },
        {
          pubkey: program.programId,
          isSigner: false,
          isWritable: false,
        },
      ])
      .rpc();

    const multisigData = await program.account.multisig.fetch(multisigPda);
    expect(multisigData.members.length).eq(1);
    expect(multisigData.members[0].toBase58()).eq(member1.publicKey.toBase58());
  });

  it("should not remove member if threshold too high", async () => {
    let proposalId = 0;
    const proposalPda = getProposalPda(
      program.programId,
      multisigPda,
      proposalId
    );

    const updateThresholdInstruction = await program.methods
      .updateThreshold(new anchor.BN(2))
      .instruction();

    const removeMemberInstruction = await program.methods
      .removeMember(member2.publicKey)
      .instruction();

    await program.methods
      .createProposal(new anchor.BN(proposalId), [
        {
          programId: updateThresholdInstruction.programId,
          data: updateThresholdInstruction.data,
          accounts: [
            {
              pubkey: multisigPda,
              isSigner: true,
              isWritable: true,
            },
            {
              pubkey: program.programId,
              isSigner: false,
              isWritable: false,
            },
          ],
        },
        {
          programId: removeMemberInstruction.programId,
          data: removeMemberInstruction.data,
          accounts: [
            {
              pubkey: multisigPda,
              isSigner: true,
              isWritable: true,
            },
            {
              pubkey: anchor.web3.SystemProgram.programId,
              isSigner: false,
              isWritable: false,
            },
            {
              pubkey: program.programId,
              isSigner: false,
              isWritable: false,
            },
          ],
        },
      ])
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .approveProposal()
      .accounts({
        multisig: multisigPda,
        proposal: proposalPda,
        signer: member1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    try {
      await program.methods
        .executeProposal()
        .accounts({
          multisig: multisigPda,
          proposal: proposalPda,
        })
        .remainingAccounts([
          {
            pubkey: multisigPda,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: program.programId,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: multisigPda,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: program.programId,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();
    } catch (e) {
      expect(e.msg).eq("Threshold too high");
      return;
    }

    assert(false, "should have failed");
  });
});
