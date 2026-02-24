"""
Teye-Contracts: Soroban Event Listener Example
----------------------------------------------
Polls the Soroban RPC for events emitted by your contract.

Install: pip install stellar-sdk requests
"""
import time
from stellar_sdk import SorobanServer
from stellar_sdk.soroban_rpc import EventFilter

RPC_URL = "https://soroban-testnet.stellar.org"
CONTRACT_ID = "C...[Insert your Teye-Contract ID here]"
POLL_INTERVAL_SECONDS = 5

def main():
    print(f"Starting Teye-Contracts Webhook Listener for {CONTRACT_ID}...")
    server = SorobanServer(RPC_URL)
    
    try:
        start_ledger = server.get_latest_ledger().sequence
        print(f"Connected. Starting at ledger: {start_ledger}")
    except Exception as e:
        print(f"Connection failed: {e}")
        return

    contract_filter = EventFilter(type="contract", contract_ids=[CONTRACT_ID])

    while True:
        try:
            current_ledger = server.get_latest_ledger().sequence
            if current_ledger >= start_ledger:
                response = server.get_events(
                    start_ledger=start_ledger,
                    filters=[contract_filter],
                    pagination={"limit": 100}
                )
                
                if response.events:
                    print(f"🔔 Found {len(response.events)} new event(s)!")
                    for event in response.events:
                        print(f" -> Processed Event [{event.id}] in Ledger {event.ledger}")
                        # Add your logic here to forward event.value.to_xdr() to your backend
                
                start_ledger = current_ledger + 1
        except Exception as e:
            print(f"Warning: {e}")
            
        time.sleep(POLL_INTERVAL_SECONDS)

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nWebhook listener stopped.")