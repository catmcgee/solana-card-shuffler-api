use card_shuffler_client::*;
use solana_sdk::pubkey::Pubkey;

#[test]
fn build_set_deck_computation_ix() {
    let payer = Pubkey::new_unique();
    let ix = set_deck_computation_ix(payer, 42);
    assert_eq!(ix.program_id, crate::ID);
    assert_eq!(ix.accounts.len(), 1);
}

#[test]
fn build_store_encrypted_hole_cards_ix() {
    let payer = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let ix = store_encrypted_hole_cards_ix(payer, owner, vec![1,2,3]);
    assert_eq!(ix.program_id, crate::ID);
    assert_eq!(ix.accounts.len(), 1);
}

#[test]
fn build_reveal_community_cards_ix() {
    let payer = Pubkey::new_unique();
    let ix = reveal_community_cards_ix(payer, vec![0,1,2]).unwrap();
    assert_eq!(ix.program_id, crate::ID);
}


