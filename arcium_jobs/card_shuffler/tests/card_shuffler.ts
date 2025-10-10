import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import { CardShuffler } from "../target/types/card_shuffler";
import { randomBytes } from "crypto";
import {
  awaitComputationFinalization,
  getArciumEnv,
  getCompDefAccOffset,
  getArciumProgAddress,
  buildFinalizeCompDefTx,
  RescueCipher,
  deserializeLE,
  getMXEAccAddress,
  getMempoolAccAddress,
  getCompDefAccAddress,
  getExecutingPoolAccAddress,
  x25519,
  getComputationAccAddress,
  getArciumAccountBaseSeed,
  getMXEPublicKey,
} from "@arcium-hq/client";
import * as fs from "fs";
import * as os from "os";
import { expect } from "chai";

// Helper function to decompress a hand from base-64 encoding
function decompressHand(
  compressedHandValue: bigint,
  handSize: number
): number[] {
  let currentHandValue = compressedHandValue;
  const cards: number[] = [];
  const numCardSlots = 11;

  for (let i = 0; i < numCardSlots; i++) {
    const card = currentHandValue % BigInt(64);
    cards.push(Number(card));
    currentHandValue >>= BigInt(6);
  }

  // Return only the actual cards based on handSize
  return cards
    .slice(0, handSize)
    .filter((card) => card <= 51)
    .reverse();
}

describe("CardShuffler", () => {
  const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.CardShuffler as Program<CardShuffler>;
  const provider = anchor.getProvider() as anchor.AnchorProvider;

  type Event = anchor.IdlEvents<(typeof program)["idl"]>;
  const awaitEvent = async <E extends keyof Event>(
    eventName: E,
    timeoutMs = 60000
  ): Promise<Event[E]> => {
    let listenerId: number;
    let timeoutId: NodeJS.Timeout;
    const event = await new Promise<Event[E]>((res, rej) => {
      listenerId = program.addEventListener(eventName as any, (event) => {
        if (timeoutId) clearTimeout(timeoutId);
        res(event);
      });
      timeoutId = setTimeout(() => {
        program.removeEventListener(listenerId);
        rej(new Error(`Event ${eventName} timed out after ${timeoutMs}ms`));
      }, timeoutMs);
    });
    await program.removeEventListener(listenerId);
    return event;
  };

  const arciumEnv = getArciumEnv();

  it("Should shuffle deck, store hole cards, reveal community cards, and change hand", async () => {
    console.log("Owner address:", owner.publicKey.toBase58());

    console.log("Initializing computation definitions...");
    await Promise.all([
      initShuffleAndDealCompDef(program as any, owner, false, false).then(
        (sig) => console.log("Shuffle/Deal CompDef Init Sig:", sig)
      ),
      initStoreHoleCardsCompDef(program as any, owner, false, false).then(
        (sig) => console.log("Store Hole Cards CompDef Init Sig:", sig)
      ),
      initRevealCommunityCompDef(program as any, owner, false, false).then(
        (sig) => console.log("Reveal Community CompDef Init Sig:", sig)
      ),
      initChangeHandCompDef(program as any, owner, false, false).then((sig) =>
        console.log("Change Hand CompDef Init Sig:", sig)
      ),
    ]);
    console.log("All computation definitions initialized.");
    await new Promise((res) => setTimeout(res, 2000));

    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const mxePublicKey = await getMXEPublicKeyWithRetry(
      provider as anchor.AnchorProvider,
      program.programId
    );

    console.log("MXE x25519 pubkey is", mxePublicKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);
    const clientNonce = randomBytes(16);

    const gameId = BigInt(Math.floor(Math.random() * 1000000));
    const mxeNonce = randomBytes(16);

    const computationOffsetInit = new anchor.BN(randomBytes(8));

    const gameIdBuffer = Buffer.alloc(8);
    gameIdBuffer.writeBigUInt64LE(gameId);

    const cardGamePDA = PublicKey.findProgramAddressSync(
      [Buffer.from("card_game"), gameIdBuffer],
      program.programId
    )[0];

    console.log(`Game ID: ${gameId}, PDA: ${cardGamePDA.toBase58()}`);

    const deckShuffledEventPromise = awaitEvent("deckShuffledEvent");
    console.log("Initializing card game...");

    const initGameSig = await program.methods
      .initializeCardGame(
        computationOffsetInit,
        new anchor.BN(gameId.toString()),
        new anchor.BN(deserializeLE(mxeNonce).toString()),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(clientNonce).toString()),
        2 // Deal 2 initial hole cards
      )
      .accountsPartial({
        computationAccount: getComputationAccAddress(
          program.programId,
          computationOffsetInit
        ),
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(program.programId),
        executingPool: getExecutingPoolAccAddress(program.programId),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(
            getCompDefAccOffset("shuffle_and_deal_deck")
          ).readUInt32LE()
        ),
        cardGame: cardGamePDA,
      })
      .signers([owner])
      .rpc({ commitment: "confirmed" });
    console.log("Initialize game TX Signature:", initGameSig);

    console.log("Waiting for shuffle/deal computation finalization...");
    const finalizeInitSig = await awaitComputationFinalization(
      provider,
      computationOffsetInit,
      program.programId,
      "confirmed"
    );
    console.log(
      "Shuffle/deal computation finalized. Signature:",
      finalizeInitSig
    );

    const deckShuffledEvent = await deckShuffledEventPromise;
    console.log("Received DeckShuffledEvent.");

    let gameState = await program.account.cardGame.fetch(cardGamePDA);

    let currentClientNonce = Uint8Array.from(
      deckShuffledEvent.holeCardsNonce.toArray("le", 16)
    );

    console.log("Current client nonce:", currentClientNonce);
    let compressedHoleCards = cipher.decrypt(
      [deckShuffledEvent.holeCards],
      currentClientNonce
    );
    let holeCards = decompressHand(
      compressedHoleCards[0],
      gameState.holeCardsSize
    );
    console.log(
      `Initial Hole Cards: ${holeCards.join(", ")} (${
        gameState.holeCardsSize
      } cards)`
    );

    expect(holeCards.length).to.equal(2);

    console.log("\n--- Storing additional hole cards ---");
    const storeHoleCardsComputationOffset = new anchor.BN(randomBytes(8));
    const holeCardsStoredEventPromise = awaitEvent("holeCardsStoredEvent");

    const storeHoleCardsSig = await program.methods
      .storeHoleCards(
        storeHoleCardsComputationOffset,
        new anchor.BN(gameId.toString()),
        1 // Add 1 more card
      )
      .accountsPartial({
        computationAccount: getComputationAccAddress(
          program.programId,
          storeHoleCardsComputationOffset
        ),
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(program.programId),
        executingPool: getExecutingPoolAccAddress(program.programId),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("store_hole_cards")).readUInt32LE()
        ),
        cardGame: cardGamePDA,
        payer: owner.publicKey,
      })
      .signers([owner])
      .rpc({ commitment: "confirmed" });
    console.log("Store Hole Cards TX Signature:", storeHoleCardsSig);

    console.log("Waiting for store hole cards computation finalization...");
    const finalizeStoreHoleCardsSig = await awaitComputationFinalization(
      provider,
      storeHoleCardsComputationOffset,
      program.programId,
      "confirmed"
    );
    console.log(
      "Store Hole Cards computation finalized. Signature:",
      finalizeStoreHoleCardsSig
    );

    const holeCardsStoredEvent = await holeCardsStoredEventPromise;
    console.log("Received HoleCardsStoredEvent.");

    gameState = await program.account.cardGame.fetch(cardGamePDA);
    currentClientNonce = Uint8Array.from(
      holeCardsStoredEvent.holeCardsNonce.toArray("le", 16)
    );
    compressedHoleCards = cipher.decrypt(
      [holeCardsStoredEvent.holeCards],
      currentClientNonce
    );
    holeCards = decompressHand(compressedHoleCards[0], gameState.holeCardsSize);
    console.log(
      `Updated Hole Cards: ${holeCards.join(", ")} (${
        gameState.holeCardsSize
      } cards)`
    );

    expect(holeCards.length).to.equal(3);

    console.log("\n--- Revealing community cards ---");
    const revealCommunityComputationOffset = new anchor.BN(randomBytes(8));
    const communityCardsRevealedEventPromise = awaitEvent(
      "communityCardsRevealedEvent"
    );

    const revealCommunitySig = await program.methods
      .revealCommunityCards(
        revealCommunityComputationOffset,
        new anchor.BN(gameId.toString()),
        3 // Reveal 3 community cards (the flop)
      )
      .accountsPartial({
        computationAccount: getComputationAccAddress(
          program.programId,
          revealCommunityComputationOffset
        ),
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(program.programId),
        executingPool: getExecutingPoolAccAddress(program.programId),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(
            getCompDefAccOffset("reveal_community_cards")
          ).readUInt32LE()
        ),
        cardGame: cardGamePDA,
        payer: owner.publicKey,
      })
      .signers([owner])
      .rpc({ commitment: "confirmed" });
    console.log("Reveal Community TX Signature:", revealCommunitySig);

    console.log("Waiting for reveal community computation finalization...");
    const finalizeRevealCommunitySig = await awaitComputationFinalization(
      provider,
      revealCommunityComputationOffset,
      program.programId,
      "confirmed"
    );
    console.log(
      "Reveal Community computation finalized. Signature:",
      finalizeRevealCommunitySig
    );

    const communityCardsRevealedEvent =
      await communityCardsRevealedEventPromise;
    console.log("Received CommunityCardsRevealedEvent.");

    const communityCards = communityCardsRevealedEvent.communityCards;
    const numRevealed = communityCardsRevealedEvent.numRevealed;
    console.log(
      `Community Cards: ${communityCards
        .slice(0, numRevealed)
        .join(", ")} (${numRevealed} cards)`
    );

    expect(numRevealed).to.equal(3);

    gameState = await program.account.cardGame.fetch(cardGamePDA);
    expect(gameState.communityCardsSize).to.equal(3);

    console.log("\n--- Changing hand for new round ---");
    const changeHandComputationOffset = new anchor.BN(randomBytes(8));
    const handChangedEventPromise = awaitEvent("handChangedEvent");

    const newNonce = randomBytes(16);

    const changeHandSig = await program.methods
      .changeHand(
        changeHandComputationOffset,
        new anchor.BN(gameId.toString()),
        new anchor.BN(deserializeLE(newNonce).toString())
      )
      .accountsPartial({
        computationAccount: getComputationAccAddress(
          program.programId,
          changeHandComputationOffset
        ),
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(program.programId),
        executingPool: getExecutingPoolAccAddress(program.programId),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("change_hand")).readUInt32LE()
        ),
        cardGame: cardGamePDA,
        payer: owner.publicKey,
      })
      .signers([owner])
      .rpc({ commitment: "confirmed" });
    console.log("Change Hand TX Signature:", changeHandSig);

    console.log("Waiting for change hand computation finalization...");
    const finalizeChangeHandSig = await awaitComputationFinalization(
      provider,
      changeHandComputationOffset,
      program.programId,
      "confirmed"
    );
    console.log(
      "Change Hand computation finalized. Signature:",
      finalizeChangeHandSig
    );

    const handChangedEvent = await handChangedEventPromise;
    console.log("Received HandChangedEvent.");

    gameState = await program.account.cardGame.fetch(cardGamePDA);
    expect(gameState.holeCardsSize).to.equal(0);
    console.log("Hand successfully reset for new round!");
  });

  // helpres
  async function initShuffleAndDealCompDef(
    program: Program<CardShuffler>,
    owner: Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    return initCompDef(
      program,
      owner,
      "shuffle_and_deal_deck",
      uploadRawCircuit,
      offchainSource
    );
  }

  async function initStoreHoleCardsCompDef(
    program: Program<CardShuffler>,
    owner: Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    return initCompDef(
      program,
      owner,
      "store_hole_cards",
      uploadRawCircuit,
      offchainSource
    );
  }

  async function initRevealCommunityCompDef(
    program: Program<CardShuffler>,
    owner: Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    return initCompDef(
      program,
      owner,
      "reveal_community_cards",
      uploadRawCircuit,
      offchainSource
    );
  }

  async function initChangeHandCompDef(
    program: Program<CardShuffler>,
    owner: Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    return initCompDef(
      program,
      owner,
      "change_hand",
      uploadRawCircuit,
      offchainSource
    );
  }

  async function initCompDef(
    program: Program<CardShuffler>,
    owner: Keypair,
    name: string,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset(name);

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log(`${name} CompDef PDA:`, compDefPDA.toBase58());

    try {
      await program.account.computationDefinitionAccount.fetch(compDefPDA);
      console.log(`${name} CompDef already initialized.`);
      return "Already Initialized";
    } catch (e) {
      // Not initialized, proceed
    }

    const methodNameMap: Record<string, string> = {
      shuffle_and_deal_deck: "initShuffleAndDealCompDef",
      store_hole_cards: "initStoreHoleCardsCompDef",
      reveal_community_cards: "initRevealCommunityCompDef",
      change_hand: "initChangeHandCompDef",
    };

    const methodName = methodNameMap[name];
    if (!methodName) {
      throw new Error(`Unknown computation definition: ${name}`);
    }

    const sig = await (program.methods as any)
      [methodName]()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
      })
      .rpc({ commitment: "confirmed" });

    if (!offchainSource) {
      console.log(`Finalizing ${name} CompDef...`);
      const finalizeTx = await buildFinalizeCompDefTx(
        provider,
        Buffer.from(offset).readUInt32LE(),
        program.programId
      );
      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
      finalizeTx.sign(owner);
      await provider.sendAndConfirm(finalizeTx, [owner], {
        commitment: "confirmed",
      });
      console.log(`${name} CompDef finalized.`);
    }
    return sig;
  }
});

async function getMXEPublicKeyWithRetry(
  provider: anchor.AnchorProvider,
  programId: PublicKey,
  maxRetries: number = 10,
  retryDelayMs: number = 500
): Promise<Uint8Array> {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const mxePublicKey = await getMXEPublicKey(provider, programId);
      if (mxePublicKey) {
        return mxePublicKey;
      }
    } catch (error) {
      console.log(`Attempt ${attempt} failed to fetch MXE public key:`, error);
    }

    if (attempt < maxRetries) {
      console.log(
        `Retrying in ${retryDelayMs}ms... (attempt ${attempt}/${maxRetries})`
      );
      await new Promise((resolve) => setTimeout(resolve, retryDelayMs));
    }
  }

  throw new Error(
    `Failed to fetch MXE public key after ${maxRetries} attempts`
  );
}

function readKpJson(path: string): anchor.web3.Keypair {
  const file = fs.readFileSync(path);
  return anchor.web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(file.toString()))
  );
}
