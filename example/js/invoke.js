/**
 * Teye-Contracts: Node.js Integration Example
 * ----------------------------------------------
 * Demonstrates how to authenticate, simulate, and submit 
 * a transaction to a Teye-Contract.
 * * Install: npm install @stellar/stellar-sdk
 */

import { Keypair, SorobanRpc, TransactionBuilder, Networks, Contract, nativeToScVal, scValToNative } from "@stellar/stellar-sdk";

const RPC_URL = "https://soroban-testnet.stellar.org";
const NETWORK_PASSPHRASE = Networks.TESTNET;
const CONTRACT_ID = "C...[Insert Teye-Contract ID]";
const SECRET_KEY = "S...[Your Server Secret Key]"; 

async function invokeTeyeContract() {
    console.log("Initializing connection...");
    const rpc = new SorobanRpc.Server(RPC_URL);
    const sourceKeypair = Keypair.fromSecret(SECRET_KEY);
    
    // 1. Fetch account state
    const account = await rpc.getAccount(sourceKeypair.publicKey());
    const contract = new Contract(CONTRACT_ID);

    // 2. Build transaction
    console.log("Building transaction...");
    const tx = new TransactionBuilder(account, {
        fee: "100", 
        networkPassphrase: NETWORK_PASSPHRASE,
    })
    .addOperation(contract.call("create_record", nativeToScVal("Example Data")))
    .setTimeout(30)
    .build();

    // 3. Simulate to calculate fees
    console.log("Simulating transaction...");
    const simulation = await rpc.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(simulation)) throw new Error(`Simulation failed: ${simulation.error}`);

    // 4. Assemble and Sign
    const assembledTx = SorobanRpc.assembleTransaction(tx, simulation);
    assembledTx.sign(sourceKeypair);

    // 5. Submit
    console.log("Submitting to network...");
    const response = await rpc.sendTransaction(assembledTx);
    if (response.status === "ERROR") throw new Error(`Submission failed: ${response.errorResultXdr}`);

    // 6. Poll for success
    console.log(`Transaction sent! Hash: ${response.hash}. Polling...`);
    let txStatus = await rpc.getTransaction(response.hash);
    
    while (txStatus.status === "NOT_FOUND") {
        await new Promise(resolve => setTimeout(resolve, 2000));
        txStatus = await rpc.getTransaction(response.hash);
    }

    if (txStatus.status === "SUCCESS") {
        console.log(`✅ Success! Returned:`, scValToNative(txStatus.returnValue));
    } else {
        console.error("❌ Transaction failed on-chain.");
    }
}

invokeTeyeContract().catch(console.error);