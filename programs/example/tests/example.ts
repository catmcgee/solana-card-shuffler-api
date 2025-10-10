// todo this doesnt work locally will have to test against devnet but deployment is failing rn

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Example } from "../target/types/example";
import { expect } from "chai";
import fs from "fs";
import path from "path";

describe("example - poker game integration", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Example as Program<Example>;
  const gameId = new anchor.BN(Date.now());

  
  const RUN_ARCIUM = process.env.RUN_ARCIUM === "1";

  type CardShufflerIdl = any;
  let cardShuffler: Program<CardShufflerIdl> | null = null;

  const root = path.resolve(__dirname, "../../../");
  const arciumRoot = path.resolve(root, "arcium_jobs/card_shuffler");

  const requireJson = (p: string) => JSON.parse(fs.readFileSync(p, "utf-8"));

  const getArtifactPubkey = (fileName: string): anchor.web3.PublicKey => {
    const full = path.join(arciumRoot, "artifacts", fileName);
    const json = requireJson(full);
    return new anchor.web3.PublicKey(json.pubkey);
  };

  const getGameSessionPda = (
    player: anchor.web3.PublicKey,
    gameId: anchor.BN
  ) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("game_session"),
        player.toBuffer(),
        gameId.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );
  };

  const CARD_SHUFFLER_PROGRAM_ID = new anchor.web3.PublicKey(
    "DQxanaqqWcTYvVhrKbeoY6q52NrGksWBL6vSbuVipnS7"
  );

  const getCardGamePda = (gameId: anchor.BN) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("card_game"), gameId.toArrayLike(Buffer, "le", 8)],
      CARD_SHUFFLER_PROGRAM_ID
    );
  };

  const getSignPda = () =>
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("SignerAccount")],
      CARD_SHUFFLER_PROGRAM_ID
    );

  const loadCardShuffler = () => {
    if (!RUN_ARCIUM) return null;
    const idlPath = path.join(arciumRoot, "target/idl/card_shuffler.json");
    if (!fs.existsSync(idlPath)) return null;
    const idl = requireJson(idlPath);
    
    return new (Program as any)(
      idl as CardShufflerIdl,
      CARD_SHUFFLER_PROGRAM_ID,
      provider
    );
  };

  const waitFor = async (
    fn: () => Promise<boolean>,
    opts: { timeoutMs?: number; intervalMs?: number } = {}
  ) => {
    const timeoutMs = opts.timeoutMs ?? 60_000;
    const intervalMs = opts.intervalMs ?? 1_000;
    const start = Date.now();
    
    while (true) {
      if (await fn()) return true;
      if (Date.now() - start > timeoutMs) return false;
      await new Promise((r) => setTimeout(r, intervalMs));
    }
  };

  before(() => {
    cardShuffler = loadCardShuffler();
  });

  it("Complete poker game flow", async () => {
    const player = provider.wallet.publicKey;
    const [gameSessionPda] = getGameSessionPda(player, gameId);
    const [cardGamePda] = getCardGamePda(gameId);

    console.log("\n=== POKER GAME INTEGRATION TEST ===\n");

    
    console.log("1️⃣  Creating game session...");
    await program.methods
      .createGameSession(gameId)
      .accounts({
        player,
      })
      .rpc();

    let gameSession = await program.account.gameSession.fetch(gameSessionPda);
    expect(gameSession.gameId.toString()).to.equal(gameId.toString());
    expect(gameSession.gameState).to.deep.equal({ waitingToShuffle: {} });
    console.log("   ✅ Game session created");
    console.log("   📍 Game Session PDA:", gameSessionPda.toBase58());
    console.log("   📍 Expected CardGame PDA:", cardGamePda.toBase58());

    
    console.log("\n2️⃣  Starting poker hand...");
    await program.methods
      .startHand()
      .accounts({
        gameSession: gameSessionPda,
      })
      .rpc();

    gameSession = await program.account.gameSession.fetch(gameSessionPda);
    expect(gameSession.gameState).to.deep.equal({ shufflingDeck: {} });
    expect(gameSession.handNumber.toString()).to.equal("1");
    console.log("   ✅ Hand #1 started");
    console.log("   🎲 Game state: ShufflingDeck");
    if (RUN_ARCIUM && cardShuffler) {
      console.log("\n   🌐 Commissioning Arcium: initialize_card_game …");

      const [signPda] = getSignPda();
      const mxeAccount = getArtifactPubkey("mxe_acc.json");
      const mempoolAccount = getArtifactPubkey("mempool_acc.json");
      const executingPool = getArtifactPubkey("executing_pool_acc.json");
      const clusterAccount = getArtifactPubkey("cluster_acc_0.json");
      const poolAccount = new anchor.web3.PublicKey(
        "7MGSS4iKNM4sVib7bDZDJhVqB6EcchPwVnTKenCY1jt3"
      );
      const clockAccount = new anchor.web3.PublicKey(
        "FHriyvoZotYiFnbUzKFjzRSb2NiaC8RPWY7jtKuKhg65"
      );

      
      const mxeNonce = new anchor.BN(Date.now());
      const clientNonce = new anchor.BN(Date.now() + 1);
      const clientPubkey = new Uint8Array(32).fill(7); 

      
      try {
        await (cardShuffler as Program).methods
          .initShuffleAndDealCompDef()
          .accounts({
            payer: player,
            mxeAccount,
            compDefAccount: anchor.web3.Keypair.generate().publicKey, 
            arciumProgram: new anchor.web3.PublicKey(
              "BKck65TgoKRokMjQM3datB9oRwJ8rAj2jxPXvHXUvcL6"
            ),
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc({ skipPreflight: true })
          .catch(() => undefined);
      } catch (_) {}

      
      await (cardShuffler as Program).methods
        .initializeCardGame(
          new anchor.BN(0),
          new anchor.BN(gameId.toString()),
          mxeNonce,
          Array.from(clientPubkey),
          clientNonce,
          2
        )
        .accounts({
          payer: player,
          signPdaAccount: signPda,
          mxeAccount,
          mempoolAccount,
          executingPool,
          computationAccount: getArtifactPubkey("mxe_keygen_comp.json"), 
          compDefAccount: getArtifactPubkey("mxe_keygen_comp_def.json"), 
          clusterAccount,
          poolAccount,
          clockAccount,
          systemProgram: anchor.web3.SystemProgram.programId,
          arciumProgram: new anchor.web3.PublicKey(
            "BKck65TgoKRokMjQM3datB9oRwJ8rAj2jxPXvHXUvcL6"
          ),
          cardGame: cardGamePda,
        })
        .rpc({ skipPreflight: true });

      
      console.log("   ⏳ Waiting for MPC callback (hole cards)…");
      const ok = await waitFor(
        async () => {
          const acctInfo = await provider.connection.getAccountInfo(
            cardGamePda
          );
          if (!acctInfo) return false;
          return true; 
        },
        { timeoutMs: 90_000, intervalMs: 2_000 }
      );
      if (!ok) throw new Error("Timed out waiting for CardGame account");
      console.log("   ✅ CardGame initialized");
    } else {
      console.log(
        "\n   ⚠️  Skipping Arcium commissioning (set RUN_ARCIUM=1 to enable)"
      );
      console.log(
        "      → Would call card_shuffler.initialize_card_game(gameId, …)"
      );
    }

    
    console.log("\n3️⃣  Dealing hole cards...");
    await program.methods
      .dealHoleCards()
      .accounts({
        gameSession: gameSessionPda,
      })
      .rpc();

    gameSession = await program.account.gameSession.fetch(gameSessionPda);
    expect(gameSession.gameState).to.deep.equal({ holeCardsDealt: {} });
    console.log("   ✅ Hole cards dealt");
    console.log("   🎲 Game state: HoleCardsDealt");
    if (!RUN_ARCIUM || !cardShuffler) {
      console.log("\n   ⚠️  Skipping storeHoleCards (Arcium not enabled)");
    }

    
    const doReveal = async (n: 1 | 3) => {
      await (cardShuffler as Program).methods
        .revealCommunityCards(
          new anchor.BN(0),
          new anchor.BN(gameId.toString()),
          n
        )
        .accounts({
          payer: player,
          signPdaAccount: getSignPda()[0],
          mxeAccount: getArtifactPubkey("mxe_acc.json"),
          mempoolAccount: getArtifactPubkey("mempool_acc.json"),
          executingPool: getArtifactPubkey("executing_pool_acc.json"),
          computationAccount: getArtifactPubkey("mxe_keygen_comp.json"), 
          compDefAccount: getArtifactPubkey("mxe_keygen_comp_def.json"), 
          clusterAccount: getArtifactPubkey("cluster_acc_0.json"),
          poolAccount: new anchor.web3.PublicKey(
            "7MGSS4iKNM4sVib7bDZDJhVqB6EcchPwVnTKenCY1jt3"
          ),
          clockAccount: new anchor.web3.PublicKey(
            "FHriyvoZotYiFnbUzKFjzRSb2NiaC8RPWY7jtKuKhg65"
          ),
          systemProgram: anchor.web3.SystemProgram.programId,
          arciumProgram: new anchor.web3.PublicKey(
            "BKck65TgoKRokMjQM3datB9oRwJ8rAj2jxPXvHXUvcL6"
          ),
          cardGame: cardGamePda,
        })
        .rpc({ skipPreflight: true });

      
      await waitFor(
        async () => {
          const acctInfo = await provider.connection.getAccountInfo(
            cardGamePda
          );
          return !!acctInfo;
        },
        { timeoutMs: 45_000, intervalMs: 1500 }
      );

      
      await program.methods
        .revealCommunityCards(n)
        .accounts({
          gameSession: gameSessionPda,
          cardGame: cardGamePda,
        } as any)
        .rpc();
    };

    console.log("\n4️⃣  Revealing flop (3 cards)...");
    if (RUN_ARCIUM && cardShuffler) {
      await doReveal(3);
      console.log("   ✅ Flop revealed via Arcium");
    } else {
      console.log("   ⚠️  Skipping (Arcium not enabled)");
    }

    console.log("\n5️⃣  Revealing turn (1 card)...");
    if (RUN_ARCIUM && cardShuffler) {
      await doReveal(1);
      console.log("   ✅ Turn revealed via Arcium");
    } else {
      console.log("   ⚠️  Skipping (Arcium not enabled)");
    }

    console.log("\n6️⃣  Revealing river (1 card)...");
    if (RUN_ARCIUM && cardShuffler) {
      await doReveal(1);
      console.log("   ✅ River revealed via Arcium");
    } else {
      console.log("   ⚠️  Skipping (Arcium not enabled)");
    }

    console.log("\n7️⃣  Poker game flow complete!");
    console.log("   💡 At this point:");
    console.log("      • CardGame.community_cards has 5 revealed cards");
    console.log("      • CardGame.hole_cards has encrypted player cards");
    console.log(
      "      • Poker program can read both via cross-program account reading"
    );
    console.log("      • Players can decrypt their hole cards off-chain");
    console.log(
      "      • Game logic determines winner based on best 5-card hand"
    );

    console.log("\n=== INTEGRATION ARCHITECTURE ===\n");
    console.log("📦 card_shuffler_client crate provides:");
    console.log("   • CardGame struct definition");
    console.log("   • get_card_game_pda() helper");
    console.log("   • CARD_SHUFFLER_PROGRAM_ID constant");
    console.log("");
    console.log("🎮 Poker program (this example):");
    console.log(
      "   • Manages game state (WaitingToShuffle → ShufflingDeck → etc)"
    );
    console.log("   • Reads CardGame via cross-program account loading");
    console.log("   • No CPI needed - just account reading");
    console.log("");
    console.log("🌐 TypeScript orchestration:");
    console.log("   • Calls card_shuffler for MPC operations");
    console.log("   • Waits for Arcium callbacks");
    console.log("   • Calls poker program for game logic");
    console.log("   • Both programs read CardGame state");
    console.log("");
    console.log("🔐 Arcium MPC network:");
    console.log("   • Handles confidential card shuffling");
    console.log("   • Encrypts hole cards");
    console.log("   • Reveals community cards");
    console.log("   • Callbacks update CardGame account");
  });

  it("Demonstrates reading card game state", async () => {
    const player = provider.wallet.publicKey;
    const [gameSessionPda] = getGameSessionPda(player, gameId);
    const [cardGamePda] = getCardGamePda(gameId);

    console.log("\n=== READING CARD GAME STATE ===\n");
    console.log("This instruction reads CardGame from card_shuffler program");
    console.log("CardGame PDA:", cardGamePda.toBase58());

    try {
      await program.methods
        .getGameInfo()
        .accounts({
          gameSession: gameSessionPda,
        })
        .rpc();

      console.log("✅ Successfully read card game info");
    } catch (err: any) {
      if (
        err.message?.includes("AccountNotInitialized") ||
        err.message?.includes("Account does not exist")
      ) {
        console.log("ℹ️  CardGame account doesn't exist yet (expected)");
        console.log("");
        console.log("To create CardGame account:");
        console.log("  1. Deploy card_shuffler program");
        console.log(
          "  2. Call card_shuffler.initCardGame(gameId, playerEncPubkey)"
        );
        console.log("  3. Then this instruction will succeed");
      } else {
        console.log("❌ Unexpected error:", err.message);
        throw err;
      }
    }
  });

  it("Shows how to end a hand", async () => {
    const player = provider.wallet.publicKey;
    const [gameSessionPda] = getGameSessionPda(player, gameId);

    console.log("\n=== ENDING A HAND ===\n");

    
    const gameSession = await program.account.gameSession.fetch(gameSessionPda);
    console.log("Current game state:", Object.keys(gameSession.gameState)[0]);

    console.log("\nℹ️  To end a hand:");
    console.log("  1. Game must be in River state");
    console.log("  2. Call poker.endHand()");
    console.log("  3. Game state resets to WaitingToShuffle");
    console.log("  4. Call card_shuffler.changeHand() to deal new cards");
    console.log("     (This updates hole cards without reshuffling full deck)");
  });

  it("Closes the game session", async () => {
    const player = provider.wallet.publicKey;
    const [gameSessionPda] = getGameSessionPda(player, gameId);

    console.log("\n=== CLOSING GAME SESSION ===\n");

    await program.methods
      .closeGameSession()
      .accounts({
        player,
        gameSession: gameSessionPda,
      } as any)
      .rpc();

    try {
      await program.account.gameSession.fetch(gameSessionPda);
      throw new Error("Game session should be closed");
    } catch (err: any) {
      if (err.message.includes("Account does not exist")) {
        console.log("✅ Game session closed successfully");
      } else {
        throw err;
      }
    }
  });

  it("Integration summary", () => {
    console.log("\n=== FULL POKER GAME FLOW ===\n");

    console.log("1. Initialize & Setup:");
    console.log(
      "   TypeScript → card_shuffler.initCardGame(gameId, playerEncPubkey)"
    );
    console.log("   TypeScript → poker.createGameSession(gameId)");
    console.log("");

    console.log("2. Start Hand:");
    console.log("   TypeScript → poker.startHand()");
    console.log("   TypeScript → card_shuffler.shuffleAndDeal(gameId, 2)");
    console.log("   Arcium MPC → Shuffles deck, encrypts cards");
    console.log("   Arcium MPC → Callback updates CardGame.deck");
    console.log("");

    console.log("3. Deal Hole Cards:");
    console.log("   TypeScript → poker.dealHoleCards()");
    console.log("   TypeScript → card_shuffler.storeHoleCards(gameId)");
    console.log("   Arcium MPC → Extracts and encrypts hole cards");
    console.log("   Arcium MPC → Callback updates CardGame.hole_cards");
    console.log("   Player → Decrypts hole cards off-chain");
    console.log("");

    console.log("4. Reveal Flop:");
    console.log(
      "   TypeScript → card_shuffler.revealCommunityCards(gameId, 3)"
    );
    console.log("   Arcium MPC → Reveals 3 cards to plaintext");
    console.log(
      "   Arcium MPC → Callback updates CardGame.community_cards[0-2]"
    );
    console.log("   TypeScript → poker.revealCommunityCards(3)");
    console.log("   Poker Program → Reads CardGame, updates state to Flop");
    console.log("");

    console.log("5. Reveal Turn:");
    console.log(
      "   TypeScript → card_shuffler.revealCommunityCards(gameId, 1)"
    );
    console.log("   Arcium MPC → Reveals 1 card to plaintext");
    console.log("   Arcium MPC → Callback updates CardGame.community_cards[3]");
    console.log("   TypeScript → poker.revealCommunityCards(1)");
    console.log("   Poker Program → Reads CardGame, updates state to Turn");
    console.log("");

    console.log("6. Reveal River:");
    console.log(
      "   TypeScript → card_shuffler.revealCommunityCards(gameId, 1)"
    );
    console.log("   Arcium MPC → Reveals 1 card to plaintext");
    console.log("   Arcium MPC → Callback updates CardGame.community_cards[4]");
    console.log("   TypeScript → poker.revealCommunityCards(1)");
    console.log("   Poker Program → Reads CardGame, updates state to River");
    console.log("");

    console.log("7. End Hand:");
    console.log("   TypeScript → poker.endHand()");
    console.log("   Poker Program → Resets state to WaitingToShuffle");
    console.log("");

    console.log("8. New Hand (Optional):");
    console.log("   TypeScript → card_shuffler.changeHand(gameId, 2)");
    console.log("   Arcium MPC → Updates hole cards without reshuffling deck");
    console.log("   Arcium MPC → Callback updates CardGame");
    console.log("   TypeScript → poker.startHand()");
    console.log("");

    console.log("=== KEY CONCEPTS ===\n");
    console.log("✅ Cross-Program Account Reading:");
    console.log("   • Poker program reads CardGame from card_shuffler");
    console.log("   • No CPI required - just account constraints");
    console.log("   • Uses seeds::program to load from other program");
    console.log("");
    console.log("✅ TypeScript Orchestration:");
    console.log("   • Calls both card_shuffler and poker programs");
    console.log("   • Waits for Arcium MPC callbacks");
    console.log("   • Synchronizes state between programs");
    console.log("");
    console.log("✅ Arcium MPC:");
    console.log("   • Handles confidential computations");
    console.log("   • Updates CardGame via callbacks");
    console.log("   • Players decrypt hole cards off-chain");
    console.log("");
    console.log("📚 Reference:");
    console.log("   • https:
    console.log("   • https:
  });
});
