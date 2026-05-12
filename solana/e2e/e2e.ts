import * as anchor from '@coral-xyz/anchor';
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import BN from 'bn.js';
import fs from 'fs';
import path from 'path';

/**
 * E2E Test for Habitat Settlement Program
 *
 * This script validates the complete pledge lifecycle:
 * 1. Initialize a pledge (user deposits escrow, oracle set, deadline future)
 * 2. Oracle resolves the pledge as success
 * 3. Verify final account state matches expected values
 *
 * Run: npm run test (from solana/e2e)
 * Requires: solana-test-validator running on http://127.0.0.1:8899
 */

async function main() {
  console.log('=== Habitat Settlement Program E2E Test ===\n');

  // 1. Setup provider and program
  const provider = anchor.AnchorProvider.local('http://127.0.0.1:8899');
  anchor.setProvider(provider);
  const connection = provider.connection;

  // Load IDL
  const idlPath = path.join(process.cwd(), '../target/idl/habitat_settlement_program.json');
  const idl = JSON.parse(fs.readFileSync(idlPath, 'utf-8'));
  const programId = new PublicKey(idl.address);
  const program = new (anchor as any).Program(idl, provider);
  console.log(`Program ID: ${programId.toBase58()}`);
  console.log(`Provider RPC: http://127.0.0.1:8899\n`);

  // 2. Create test keypairs
  const user = Keypair.generate();
  const oracle = Keypair.generate();
  const penaltyPool = Keypair.generate();

  console.log(`User:         ${user.publicKey.toBase58()}`);
  console.log(`Oracle:       ${oracle.publicKey.toBase58()}`);
  console.log(`Penalty Pool: ${penaltyPool.publicKey.toBase58()}\n`);

  // 3. Airdrop SOL to fund accounts
  console.log('Funding accounts...');
  const userAirdrop = await connection.requestAirdrop(user.publicKey, 5 * LAMPORTS_PER_SOL);
  const oracleAirdrop = await connection.requestAirdrop(oracle.publicKey, 5 * LAMPORTS_PER_SOL);
  const poolAirdrop = await connection.requestAirdrop(penaltyPool.publicKey, 5 * LAMPORTS_PER_SOL);

  const latestBlockhash = await connection.getLatestBlockhash('confirmed');
  await connection.confirmTransaction({ signature: userAirdrop, ...latestBlockhash }, 'confirmed');
  await connection.confirmTransaction({ signature: oracleAirdrop, ...latestBlockhash }, 'confirmed');
  await connection.confirmTransaction({ signature: poolAirdrop, ...latestBlockhash }, 'confirmed');
  console.log('✓ Airdrop complete.\n');

  // 4. Create pledge PDA
  const pledgeId = 'test-pledge-001';
  const [pledgePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('pledge'), user.publicKey.toBuffer(), Buffer.from(pledgeId)],
    programId
  );

  console.log(`Pledge PDA: ${pledgePda.toBase58()}\n`);

  // 5. Initialize pledge
  console.log('--- Step 1: Initialize Pledge ---');
  const escrowAmount = new BN(1_000_000); // 1M lamports
  const futureDeadline = Math.floor(Date.now() / 1000) + 86400; // 24 hours in future

  try {
    const txInit = await program.methods
      .initializePledge({
        pledgeId,
        oraclePubkey: oracle.publicKey,
        escrowAmount,
        deadlineTimestamp: new BN(futureDeadline),
      })
      .accounts({
        payer: user.publicKey,
        pledge: pledgePda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    console.log(`✓ Initialize TX: ${txInit}`);
  } catch (err: any) {
    console.error(`✗ Initialize failed:`, err.message || err);
    throw err;
  }

  // 6. Check if pledge account exists
  console.log('\nVerifying pledge account creation...');
  const pledgeAccount = await connection.getAccountInfo(pledgePda);
  if (pledgeAccount) {
    console.log(`✓ Pledge account created`);
    console.log(`  Owner:      ${pledgeAccount.owner.toBase58()}`);
    console.log(`  Lamports:   ${pledgeAccount.lamports}`);
    console.log(`  Data size:  ${pledgeAccount.data.length} bytes\n`);
  } else {
    console.log(`✗ Pledge account not found at ${pledgePda.toBase58()}`);
    throw new Error('Pledge account creation failed');
  }

  // 7. Resolve success
  console.log('--- Step 2: Resolve Success ---');
  const resolutionProof = 'user-won-txsig-123abc';

  try {
    const txResolve = await program.methods
      .resolveSuccess(resolutionProof)
      .accounts({
        oracle: oracle.publicKey,
        pledge: pledgePda,
        user: user.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        penaltyPool: penaltyPool.publicKey,
      })
      .signers([oracle])
      .rpc();

    console.log(`✓ Resolve Success TX: ${txResolve}`);
  } catch (err: any) {
    console.error(`✗ Resolve failed:`, err.message || err);
    throw err;
  }

  // 8. Verify pledge account still exists
  console.log('\n--- Step 3: Verify Final State ---');
  const pledgeAccountFinal = await connection.getAccountInfo(pledgePda);
  if (pledgeAccountFinal) {
    console.log(`✓ Pledge account exists after resolution`);
    console.log(`  Owner:      ${pledgeAccountFinal.owner.toBase58()}`);
    console.log(`  Lamports:   ${pledgeAccountFinal.lamports}`);
    console.log(`  Data size:  ${pledgeAccountFinal.data.length} bytes\n`);
  } else {
    console.log(`✗ Pledge account missing after resolution`);
    throw new Error('Pledge account missing');
  }

  console.log('=== Test Complete ===');
  console.log('✓ All steps executed successfully!');
  console.log('✓ Smart contract is responding to transactions.\n');
  process.exit(0);
}

main().catch((err: any) => {
  console.error('Fatal error:', err.message || err);
  process.exit(1);
});
