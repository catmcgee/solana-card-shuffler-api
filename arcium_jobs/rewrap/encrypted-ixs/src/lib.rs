use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

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
    pub fn transfer_hand(
        hand_ctxt: Enc<Shared, Hand>,
        new_owner: Shared,
    ) -> Enc<Shared, Hand> {
        let hand = hand_ctxt.to_arcis();
        new_owner.from_arcis(hand)
    }

    #[instruction]
    pub fn transfer_card(
        card_ctxt: Enc<Shared, u8>,
        new_owner: Shared,
    ) -> Enc<Shared, u8> {
        let card = card_ctxt.to_arcis();
        new_owner.from_arcis(card)
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
}
