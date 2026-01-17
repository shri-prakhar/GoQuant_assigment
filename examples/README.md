# Collateral Vault API Examples

This directory contains examples showing how to interact with the Collateral Vault API.

## JavaScript/Node.js Example

```javascript
const API_BASE = "http://localhost:3000/api/v1";

// Initialize a new vault
async function initializeVault(vaultPubkey, ownerPubkey, tokenAccount) {
  const response = await fetch(`${API_BASE}/vault/initialize`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      vault_pubkey: vaultPubkey,
      owner_pubkey: ownerPubkey,
      token_account: tokenAccount,
    }),
  });

  const result = await response.json();
  if (result.success) {
    console.log("Vault initialized:", result.data);
  } else {
    console.error("Error:", result.error);
  }
}

// Get vault balance
async function getVaultBalance(vaultPubkey) {
  const response = await fetch(`${API_BASE}/vault/balance/${vaultPubkey}`);
  const result = await response.json();

  if (result.success) {
    console.log("Vault balance:", result.data);
  } else {
    console.error("Error:", result.error);
  }
}

// Deposit collateral
async function depositCollateral(vaultPubkey, amount, txSignature) {
  const response = await fetch(`${API_BASE}/vault/deposit`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      vault_pubkey: vaultPubkey,
      amount: amount,
      tx_signature: txSignature,
    }),
  });

  const result = await response.json();
  if (result.success) {
    console.log("Deposit processed:", result.data);
  } else {
    console.error("Error:", result.error);
  }
}

// Withdraw collateral
async function withdrawCollateral(vaultPubkey, amount, txSignature) {
  const response = await fetch(`${API_BASE}/vault/withdraw`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      vault_pubkey: vaultPubkey,
      amount: amount,
      tx_signature: txSignature,
    }),
  });

  const result = await response.json();
  if (result.success) {
    console.log("Withdrawal processed:", result.data);
  } else {
    console.error("Error:", result.error);
  }
}

// Lock collateral
async function lockCollateral(vaultPubkey, amount, txSignature) {
  const response = await fetch(`${API_BASE}/vault/lock`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      vault_pubkey: vaultPubkey,
      amount: amount,
      tx_signature: txSignature,
    }),
  });

  const result = await response.json();
  if (result.success) {
    console.log("Collateral locked:", result.data);
  } else {
    console.error("Error:", result.error);
  }
}

// WebSocket real-time updates
function connectWebSocket() {
  const ws = new WebSocket("ws://localhost:3000/ws");

  ws.onopen = () => {
    console.log("Connected to WebSocket");
  };

  ws.onmessage = (event) => {
    const update = JSON.parse(event.data);
    console.log("Real-time update:", update);

    switch (update.type) {
      case "balance_update":
        console.log("Balance changed for vault:", update.vault_pubkey);
        break;
      case "deposit":
        console.log("Deposit detected:", update);
        break;
      case "withdrawal":
        console.log("Withdrawal detected:", update);
        break;
      case "lock":
        console.log("Collateral locked:", update);
        break;
      case "unlock":
        console.log("Collateral unlocked:", update);
        break;
    }
  };

  ws.onclose = () => {
    console.log("WebSocket connection closed");
  };

  return ws;
}

// Example usage
async function example() {
  const vaultPubkey = "4rL4RCWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY5Mj";
  const ownerPubkey = "4rL4RCWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY5Mj";
  const tokenAccount = "BYLfz8RQMYE7A5FwL2fVn7RZnYNqh82cBzJpXu9hS3Rq";

  try {
    // Initialize vault
    await initializeVault(vaultPubkey, ownerPubkey, tokenAccount);

    // Check balance
    await getVaultBalance(vaultPubkey);

    // Connect to real-time updates
    const ws = connectWebSocket();

    // Simulate operations (in real usage, these would be actual blockchain transactions)
    // await depositCollateral(vaultPubkey, 1000000, 'some_tx_signature');
    // await lockCollateral(vaultPubkey, 500000, 'some_tx_signature');
  } catch (error) {
    console.error("Example failed:", error);
  }
}

example();
```

## Python Example

```python
import asyncio
import websockets
import json
import aiohttp

API_BASE = 'http://localhost:3000/api/v1'

async def initialize_vault(session, vault_pubkey, owner_pubkey, token_account):
    """Initialize a new vault"""
    async with session.post(f'{API_BASE}/vault/initialize',
                           json={
                               'vault_pubkey': vault_pubkey,
                               'owner_pubkey': owner_pubkey,
                               'token_account': token_account
                           }) as response:
        result = await response.json()
        if result['success']:
            print('Vault initialized:', result['data'])
        else:
            print('Error:', result['error'])

async def get_vault_balance(session, vault_pubkey):
    """Get vault balance"""
    async with session.get(f'{API_BASE}/vault/balance/{vault_pubkey}') as response:
        result = await response.json()
        if result['success']:
            print('Vault balance:', result['data'])
        else:
            print('Error:', result['error'])

async def websocket_listener():
    """Listen for real-time WebSocket updates"""
    uri = "ws://localhost:3000/ws"
    async with websockets.connect(uri) as websocket:
        print("Connected to WebSocket")

        async for message in websocket:
            update = json.loads(message)
            print(f"Real-time update: {update}")

            if update['type'] == 'balance_update':
                print(f"Balance changed for vault: {update['vault_pubkey']}")
            elif update['type'] == 'deposit':
                print(f"Deposit detected: {update}")

async def main():
    vault_pubkey = '4rL4RCWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY5Mj'
    owner_pubkey = '4rL4RCWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY5Mj'
    token_account = 'BYLfz8RQMYE7A5FwL2fVn7RZnYNqh82cBzJpXu9hS3Rq'

    async with aiohttp.ClientSession() as session:
        # Initialize vault
        await initialize_vault(session, vault_pubkey, owner_pubkey, token_account)

        # Get balance
        await get_vault_balance(session, vault_pubkey)

        # Start WebSocket listener
        await websocket_listener()

if __name__ == '__main__':
    asyncio.run(main())
```

## cURL Examples

### Health Check

```bash
curl http://localhost:3000/health
```

### Initialize Vault

```bash
curl -X POST http://localhost:3000/api/v1/vault/initialize \
  -H "Content-Type: application/json" \
  -d '{
    "vault_pubkey": "4rL4RCWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY5Mj",
    "owner_pubkey": "4rL4RCWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY5Mj",
    "token_account": "BYLfz8RQMYE7A5FwL2fVn7RZnYNqh82cBzJpXu9hS3Rq"
  }'
```

### Get Vault Balance

```bash
curl http://localhost:3000/api/v1/vault/balance/4rL4RCWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY5Mj
```

### Deposit Collateral

```bash
curl -X POST http://localhost:3000/api/v1/vault/deposit \
  -H "Content-Type: application/json" \
  -d '{
    "vault_pubkey": "4rL4RCWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY5Mj",
    "amount": 1000000,
    "tx_signature": "some_transaction_signature"
  }'
```

### Build Deposit Transaction

```bash
curl -X POST http://localhost:3000/api/v1/transaction/deposit \
  -H "Content-Type: application/json" \
  -d '{
    "vault_pubkey": "4rL4RCWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY5Mj",
    "user_pubkey": "4rL4RCWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY5Mj",
    "amount": 1000000
  }'
```

### Get TVL Stats

```bash
curl http://localhost:3000/api/v1/vault/tvl
```

### Get Metrics

```bash
curl http://localhost:3000/metrics
```
