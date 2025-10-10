use anchor_lang::prelude::*;

/// The card_shuffler program ID
/// This must match the ID in the deployed card_shuffler program
pub const CARD_SHUFFLER_PROGRAM_ID: Pubkey = solana_program::pubkey!("DQxanaqqWcTYvVhrKbeoY6q52NrGksWBL6vSbuVipnS7");

pub const MAX_HOLE_CARDS: usize = 11;
pub const MAX_COMMUNITY_CARDS: usize = 5;
pub const EMPTY_CARD_MARKER: u8 = 53;

/// Helper function to derive the CardGame PDA from the card_shuffler program
pub fn get_card_game_pda(game_id: u64) -> (Pubkey, u8) {
    let game_id_bytes = game_id.to_le_bytes();
    Pubkey::find_program_address(
        &[b"card_game", game_id_bytes.as_ref()],
        &CARD_SHUFFLER_PROGRAM_ID,
    )
}

/// Represents a card game session with encrypted deck and hands.
/// This is the main account managed by the card_shuffler program.

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CardGame {
    /// Encrypted deck split into 3 chunks (52 cards encoded in base-64)
    pub deck: [[u8; 32]; 3],
    /// Cryptographic nonce for deck encryption
    pub deck_nonce: u128,
    /// Player's encrypted hole cards
    pub hole_cards: [u8; 32],
    /// Cryptographic nonce for hole cards encryption
    pub hole_cards_nonce: u128,
    /// Number of hole cards currently held
    pub hole_cards_size: u8,
    /// Revealed community cards (plaintext)
    pub community_cards: [u8; 5],
    /// Number of community cards revealed
    pub community_cards_size: u8,
    /// Total number of cards dealt from the deck
    pub cards_dealt: u8,
    /// Unique identifier for this game session
    pub game_id: u64,
    /// Solana public key of the player
    pub player_pubkey: Pubkey,
    /// Player's encryption public key for MPC operations
    pub player_enc_pubkey: [u8; 32],
    /// PDA bump seed
    pub bump: u8,
}

impl anchor_lang::AccountSerialize for CardGame {
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
        AnchorSerialize::serialize(self, writer)
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotSerialize.into())
    }
}

impl anchor_lang::AccountDeserialize for CardGame {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        AnchorDeserialize::deserialize(buf)
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
    }
}

impl anchor_lang::Owner for CardGame {
    fn owner() -> Pubkey {
        CARD_SHUFFLER_PROGRAM_ID
    }
}

impl anchor_lang::Discriminator for CardGame {
    const DISCRIMINATOR: &'static [u8] = &[197, 251, 174, 61, 185, 100, 214, 215];
}
