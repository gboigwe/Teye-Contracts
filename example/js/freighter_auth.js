/**
 * Teye-Contracts: Frontend Authentication Flow Example
 * ----------------------------------------------------
 * Demonstrates how a Web3 frontend connects to the user's Freighter wallet 
 * to sign a transaction interacting with Teye-Contracts.
 * * Install: npm install @stellar/freighter-api
 */

import { isConnected, getPublicKey, signTransaction } from "@stellar/freighter-api";

/**
 * Step 1: Connect the Wallet
 */
export async function connectWallet() {
  console.log("Checking if Freighter is installed...");
  
  if (!(await isConnected())) {
    alert("Freighter wallet is not installed! Please install it from freighter.app");
    return null;
  }

  try {
    const publicKey = await getPublicKey();
    console.log(`Connected successfully! User Address: ${publicKey}`);
    return publicKey;
  } catch (error) {
    console.error("User rejected the connection request:", error);
    return null;
  }
}

/**
 * Step 2: Sign the Transaction
 * @param {string} transactionXdr - The base64 XDR string of the built transaction
 * @param {string} network - The network passphrase (e.g., 'TESTNET' or 'PUBLIC')
 */
export async function signWithFreighter(transactionXdr, network = "TESTNET") {
  try {
    console.log("Requesting user signature via Freighter...");
    const signedTxXdr = await signTransaction(transactionXdr, { network: network });
    console.log("Transaction successfully signed by the user!");
    return signedTxXdr;
  } catch (error) {
    console.error("Signature request failed or was rejected by user:", error);
    throw error;
  }
}