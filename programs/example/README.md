# Example Program

Integration example showing how to use the `card_shuffler_client` crate in your Anchor program.

## Overview

This example demonstrates how to build a poker/card game program that reads CardGame state from the card_shuffler program. It shows the complete integration pattern for building confidential card games on Solana using Arcium MPC.

## What This Example Shows

1. **Creating Game Sessions**: Initialize game state that tracks card_shuffler games
2. **Cross-Program Account Reading**: Read CardGame accounts from card_shuffler program
3. **Accessing Card State**: View encrypted hole cards and plaintext community cards
4. **Full Integration Flow**: Complete poker game flow documentation

## Program Structure

### Instructions

- `create_game_session(game_id)` - Create a game session that tracks a card_shuffler game
- `get_game_info()` - Read CardGame state from card_shuffler program
- `close_game_session()` - Close and cleanup game session

### Accounts

- `GameSession` - Tracks game state for your poker program

## Running the Example

### Prerequisites

- Anchor CLI installed
- Solana CLI installed
- Local validator running (or devnet access)
- card_shuffler program deployed

### Build

```bash
anchor build
```

### Test

```bash
# Start local validator in another terminal
solana-test-validator

# Run tests
anchor test --skip-local-validator
```

### Test Output

The tests demonstrate:

1. ✅ Creating a game session
2. ✅ Reading card game info (shows expected behavior when CardGame doesn't exist yet)
3. ✅ Complete poker game flow documentation
4. ✅ Closing game session
5. ✅ Integration summary

## Code Examples

### Reading CardGame State

```rust
pub fn get_game_info(ctx: Context<GetGameInfo>) -> Result<()> {
    let card_game = &ctx.accounts.card_game;

    msg!("Card Game State (from card_shuffler):");
    msg!("  Hole cards size: {}", card_game.hole_cards_size);
    msg!("  Community cards size: {}", card_game.community_cards_size);
    msg!("  Cards dealt: {}", card_game.cards_dealt);

    Ok(())
}
```

### Cross-Program Account Loading

```rust
#[derive(Accounts)]
pub struct GetGameInfo<'info> {
    pub game_session: Account<'info, GameSession>,

    /// The CardGame account from the card_shuffler program
    #[account(
        seeds = [b"card_game", game_session.game_id.to_le_bytes().as_ref()],
        bump,
        seeds::program = CARD_SHUFFLER_PROGRAM_ID,
    )]
    pub card_game: Account<'info, CardGame>,
}
```

## Integration Flow

### Complete Poker Game Flow

1. **TypeScript: Initialize CardGame**
   ```typescript
   await cardShuffler.methods.initCardGame(gameId, playerEncPubkey).rpc();
   ```

2. **TypeScript: Create Game Session**
   ```typescript
   await pokerProgram.methods.createGameSession(gameId).rpc();
   ```

3. **TypeScript: Shuffle and Deal**
   ```typescript
   await cardShuffler.methods.shuffleAndDeal(gameId, numHoleCards).rpc();
   // Arcium MPC network processes → Callback updates CardGame.deck
   ```

4. **TypeScript: Store Hole Cards**
   ```typescript
   await cardShuffler.methods.storeHoleCards(gameId).rpc();
   // Arcium MPC network processes → Callback updates CardGame.hole_cards
   ```

5. **TypeScript: Reveal Community Cards (Flop)**
   ```typescript
   await cardShuffler.methods.revealCommunityCards(gameId, 3).rpc();
   // Arcium MPC network processes → Callback updates CardGame.community_cards
   ```

6. **Rust: Read Card State**
   ```typescript
   await pokerProgram.methods.getGameInfo().rpc();
   // Cross-program account reading - no CPI needed
   ```

7. **TypeScript: Reveal Turn**
   ```typescript
   await cardShuffler.methods.revealCommunityCards(gameId, 1).rpc();
   ```

8. **TypeScript: Reveal River**
   ```typescript
   await cardShuffler.methods.revealCommunityCards(gameId, 1).rpc();
   ```

9. **TypeScript: Change Hand (New Round)**
   ```typescript
   await cardShuffler.methods.changeHand(gameId, newNumHoleCards).rpc();
   // Arcium MPC network processes → Updates hand without reshuffling
   ```

10. **TypeScript: Close Session**
    ```typescript
    await pokerProgram.methods.closeGameSession().rpc();
    ```

## Architecture

### No CPI Required ✅

This example uses **cross-program account reading**, not CPI:

- ✅ Reads CardGame accounts from card_shuffler program
- ✅ Accesses encrypted hole_cards and plaintext community_cards
- ✅ Implements game logic based on card state
- ❌ Does NOT call card_shuffler instructions via CPI

### Why No CPI?

Arcium programs already CPI into the Arcium system program for MPC operations. Your poker program doesn't need to CPI into card_shuffler - it just reads the CardGame account state.

### TypeScript Orchestration

The TypeScript client orchestrates calls to both programs:

```
TypeScript Client
    ├─→ card_shuffler program (MPC operations)
    │       └─→ Arcium system program (MPC network)
    │
    └─→ poker program (game logic)
            └─→ Reads CardGame account
```

## Dependencies

### Cargo.toml

```toml
[dependencies]
anchor-lang = "0.31.1"
card_shuffler_client = { path = "../../crates/card_shuffler_client" }
```

### package.json

```json
{
  "dependencies": {
    "@coral-xyz/anchor": "^0.31.1",
    "@solana/web3.js": "^1.95.8"
  }
}
```

## Key Concepts

### CardGame Account

The CardGame account is owned by the card_shuffler program and contains:

- `deck`: Encrypted deck (3 × 32 bytes)
- `hole_cards`: Encrypted hole cards (32 bytes)
- `community_cards`: Plaintext community cards (5 × u8)
- `cards_dealt`: Number of cards dealt from deck
- `game_id`: Unique game identifier
- `player_pubkey`: Player's Solana public key
- `player_enc_pubkey`: Player's encryption public key for MPC

### PDA Seeds

```rust
// CardGame PDA (owned by card_shuffler program)
seeds = [b"card_game", game_id.to_le_bytes().as_ref()]
program = CARD_SHUFFLER_PROGRAM_ID

// GameSession PDA (owned by your poker program)
seeds = [b"game_session", player.key().as_ref(), game_id.to_le_bytes().as_ref()]
program = YOUR_POKER_PROGRAM_ID
```

### MPC Callbacks

When you call card_shuffler instructions:

1. Instruction queues computation on Arcium network
2. MPC nodes process confidentially
3. Callback instruction updates CardGame account
4. Your program reads updated state

## Testing Locally

### With card_shuffler Deployed

If you have card_shuffler deployed locally:

```bash
# Terminal 1: Start validator
solana-test-validator

# Terminal 2: Deploy card_shuffler
cd arcium_jobs/card_shuffler
arcium deploy

# Terminal 3: Run example tests
cd programs/example
anchor test --skip-local-validator
```

### Without card_shuffler

The tests will show expected behavior when CardGame accounts don't exist yet:

```bash
anchor test --skip-local-validator
```

## Next Steps

1. **Deploy to Devnet**: Test with real Arcium MPC network
2. **Add Game Logic**: Implement betting, hand evaluation, etc.
3. **Multi-Player Support**: Extend to support multiple players per game
4. **UI Integration**: Build frontend using Solana wallet adapters

## References

- [card_shuffler_client crate](../../crates/card_shuffler_client/README.md)
- [Arcium Documentation](https://docs.arcium.com)
- [Poker Example](https://github.com/brimigs/poker)
- [Anchor Framework](https://www.anchor-lang.com/)

## License

See LICENSE file in repository root.
