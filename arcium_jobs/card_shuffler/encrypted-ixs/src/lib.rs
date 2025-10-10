// heavily inspired by arcium's blackjack example

use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    /// Standard 52-card deck represented as indices 0-51
    const INITIAL_DECK: [u8; 52] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51,
    ];

    /// Powers of 64 used for encoding cards into u128 values.
    /// Each card takes 6 bits (values 0-63), so we can pack multiple cards efficiently.
    /// This array contains 64^i for i in 0..21, allowing us to encode up to 21 cards per u128.
    const POWS_OF_SIXTY_FOUR: [u128; 21] = [
        1,
        64,
        4096,
        262144,
        16777216,
        1073741824,
        68719476736,
        4398046511104,
        281474976710656,
        18014398509481984,
        1152921504606846976,
        73786976294838206464,
        4722366482869645213696,
        302231454903657293676544,
        19342813113834066795298816,
        1237940039285380274899124224,
        79228162514264337593543950336,
        5070602400912917605986812821504,
        324518553658426726783156020576256,
        20769187434139310514121985316880384,
        1329227995784915872903807060280344576,
    ];

    /// Represents a full 52-card deck encoded into three u128 values for efficiency.
    ///
    /// Each card is represented by 6 bits (0-63 range), allowing us to pack:
    /// - Cards 0-20 in card_one (21 cards × 6 bits = 126 bits < 128 bits)
    /// - Cards 21-41 in card_two (21 cards × 6 bits = 126 bits < 128 bits)
    /// - Cards 42-51 in card_three (10 cards × 6 bits = 60 bits < 128 bits)
    pub struct Deck {
        pub card_one: u128,
        pub card_two: u128,
        pub card_three: u128,
    }

    impl Deck {
        /// Converts a 52-card array into the packed Deck representation.
        /// Uses base-64 encoding where each card index is treated as a digit in base 64.
        pub fn from_array(array: [u8; 52]) -> Deck {
            let mut card_one = 0;
            for i in 0..21 {
                card_one += POWS_OF_SIXTY_FOUR[i] * array[i] as u128;
            }

            let mut card_two = 0;
            for i in 21..42 {
                card_two += POWS_OF_SIXTY_FOUR[i - 21] * array[i] as u128;
            }

            let mut card_three = 0;
            for i in 42..52 {
                card_three += POWS_OF_SIXTY_FOUR[i - 42] * array[i] as u128;
            }

            Deck {
                card_one,
                card_two,
                card_three,
            }
        }

        /// Converts the packed Deck representation back to a 52-card array.
        /// Reverses the base-64 encoding by extracting 6 bits at a time.
        fn to_array(&self) -> [u8; 52] {
            let mut card_one = self.card_one;
            let mut card_two = self.card_two;
            let mut card_three = self.card_three;

            let mut bytes = [0u8; 52];
            for i in 0..21 {
                bytes[i] = (card_one % 64) as u8;
                bytes[i + 21] = (card_two % 64) as u8;
                card_one >>= 6;
                card_two >>= 6;
            }

            for i in 42..52 {
                bytes[i] = (card_three % 64) as u8;
                card_three >>= 6;
            }

            bytes
        }
    }

    /// Represents a hand of up to 11 cards encoded into a single u128.
    /// Uses the same base-64 encoding scheme as Deck.
    pub struct Hand {
        pub cards: u128,
    }

    impl Hand {
        pub fn from_array(array: [u8; 11]) -> Hand {
            let mut cards = 0;
            for i in 0..11 {
                cards += POWS_OF_SIXTY_FOUR[i] * array[i] as u128;
            }

            Hand { cards }
        }

        fn to_array(&self) -> [u8; 11] {
            let mut cards = self.cards;

            let mut bytes = [0u8; 11];
            for i in 0..11 {
                bytes[i] = (cards % 64) as u8;
                cards >>= 6;
            }

            bytes
        }
    }

    /// Shuffles a deck and deals initial hole cards.
    ///
    /// Returns:
    /// - Encrypted deck (for MXE to manage subsequent draws)
    /// - Encrypted hole cards for player (shared encryption with client)
    /// - Number of cards dealt (to track deck position)
    #[instruction]
    pub fn shuffle_and_deal_deck(
        mxe: Mxe,
        client: Shared,
        num_hole_cards: u8,
    ) -> (
        Enc<Mxe, Deck>,    // Encrypted deck
        Enc<Shared, Hand>, // Player hole cards
        u8,                // Number of cards dealt
    ) {
        // Shuffle the deck using Arcium's RNG
        let mut deck = INITIAL_DECK;
        ArcisRNG::shuffle(&mut deck);

        // Encode the shuffled deck
        let encrypted_deck = mxe.from_arcis(Deck::from_array(deck));

        // Deal hole cards to player
        let mut hole_cards = [53; 11]; // 53 = empty card marker
        for i in 0..11 {
            if i < num_hole_cards {
                hole_cards[i as usize] = deck[i as usize];
            }
        }

        let encrypted_hole_cards = client.from_arcis(Hand::from_array(hole_cards));

        (encrypted_deck, encrypted_hole_cards, num_hole_cards)
    }

    /// Stores encrypted hole cards for a player.
    /// Takes an existing hand and adds new cards from the deck.
    ///
    /// Returns:
    /// - Updated encrypted hand
    /// - Number of cards now in hand
    #[instruction]
    pub fn store_hole_cards(
        deck_ctxt: Enc<Mxe, Deck>,
        existing_hand_ctxt: Enc<Shared, Hand>,
        existing_hand_size: u8,
        num_new_cards: u8,
        cards_already_dealt: u8,
    ) -> (Enc<Shared, Hand>, u8) {
        let deck = deck_ctxt.to_arcis().to_array();
        let mut hand = existing_hand_ctxt.to_arcis().to_array();

        // Add new cards to the hand
        let mut cards_added = 0;
        for i in 0..11 {
            if i < num_new_cards && existing_hand_size + i < 11 {
                let deck_index = (cards_already_dealt + i) as usize;
                if deck_index < 52 {
                    hand[(existing_hand_size + i) as usize] = deck[deck_index];
                    cards_added += 1;
                }
            }
        }

        let updated_hand = existing_hand_ctxt
            .owner
            .from_arcis(Hand::from_array(hand));

        (updated_hand, existing_hand_size + cards_added)
    }

    /// Reveals N community cards from the deck
    /// These cards are revealed as plaintext
    ///
    /// Returns:
    /// - Array of revealed community cards (up to 5 cards for poker)
    /// - Number of cards revealed
    #[instruction]
    pub fn reveal_community_cards(
        deck_ctxt: Enc<Mxe, Deck>,
        num_cards_to_reveal: u8,
        cards_already_dealt: u8,
    ) -> ([u8; 5], u8) {
        let deck = deck_ctxt.to_arcis().to_array();

        let mut community_cards = [53u8; 5]; 
        let mut cards_revealed = 0;

        for i in 0..5 {
            if i < num_cards_to_reveal {
                let deck_index = (cards_already_dealt + i) as usize;
                if deck_index < 52 {
                    community_cards[i as usize] = deck[deck_index];
                    cards_revealed += 1;
                }
            }
        }

        // Reveal the cards (make them public)
        let revealed_cards = [
            community_cards[0].reveal(),
            community_cards[1].reveal(),
            community_cards[2].reveal(),
            community_cards[3].reveal(),
            community_cards[4].reveal(),
        ];

        (revealed_cards, cards_revealed.reveal())
    }

    /// Changes/resets a hand for a new round
    /// Creates a fresh empty hand encrypted for the client
    ///
    /// Returns:
    /// - New empty encrypted hand
    #[instruction]
    pub fn change_hand(client: Shared) -> Enc<Shared, Hand> {
        let empty_hand = [53; 11]; // All cards set to empty marker
        client.from_arcis(Hand::from_array(empty_hand))
    }
}
