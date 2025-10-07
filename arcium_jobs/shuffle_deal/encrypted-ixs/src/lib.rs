use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    const INITIAL_DECK: [u8; 52] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51,
    ];

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

    pub struct Deck {
        pub card_one: u128,
        pub card_two: u128,
        pub card_three: u128,
    }

    impl Deck {
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

    #[instruction]
    pub fn shuffle_and_deal(
        mxe: Mxe,
        player1: Shared,
        player2: Shared,
        cards_per_player: u8,
    ) -> (Enc<Mxe, Deck>, Enc<Shared, Hand>, Enc<Shared, Hand>) {
        let mut deck = INITIAL_DECK;
        ArcisRNG::shuffle(&mut deck);

        let deck_enc = mxe.from_arcis(Deck::from_array(deck));

        let mut hand1 = [53u8; 11];
        let mut hand2 = [53u8; 11];

        let cpp = cards_per_player as usize;
        for r in 0..11 {
            if r < cpp {
                hand1[r] = deck[r * 2];
                hand2[r] = deck[r * 2 + 1];
            }
        }

        let hand1_enc = player1.from_arcis(Hand::from_array(hand1));
        let hand2_enc = player2.from_arcis(Hand::from_array(hand2));

        (deck_enc, hand1_enc, hand2_enc)
    }

    #[instruction]
    pub fn deal_card(
        deck_ctxt: Enc<Mxe, Deck>,
        hand_ctxt: Enc<Shared, Hand>,
        card_index: u8,
        hand_size: u8,
    ) -> Enc<Shared, Hand> {
        let deck = deck_ctxt.to_arcis().to_array();
        let mut hand = hand_ctxt.to_arcis().to_array();

        hand[hand_size as usize] = deck[card_index as usize];

        hand_ctxt.owner.from_arcis(Hand::from_array(hand))
    }

    #[instruction]
    pub fn exchange_hands(
        hand1_ctxt: Enc<Shared, Hand>,
        hand2_ctxt: Enc<Shared, Hand>,
    ) -> (Enc<Shared, Hand>, Enc<Shared, Hand>) {
        let hand1 = hand1_ctxt.to_arcis();
        let hand2 = hand2_ctxt.to_arcis();

        (
            hand2_ctxt.owner.from_arcis(hand1),
            hand1_ctxt.owner.from_arcis(hand2),
        )
    }

    #[instruction]
    pub fn reveal_card(
        deck_ctxt: Enc<Mxe, Deck>,
        card_index: u8,
    ) -> u8 {
        let deck = deck_ctxt.to_arcis().to_array();
        deck[card_index as usize].reveal()
    }
}
