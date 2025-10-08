use card_shuffler_client::arcium_flows::*;
use solana_sdk::pubkey::Pubkey;

#[test]
fn build_shuffle_and_deal_ix() {
    let payer = Pubkey::new_unique();
    let ix = shuffle_and_deal_ix(
        payer,
        Pubkey::new_unique(), // sign_pda
        Pubkey::new_unique(), // mxe
        Pubkey::new_unique(), // mempool
        Pubkey::new_unique(), // executing_pool
        Pubkey::new_unique(), // computation
        Pubkey::new_unique(), // comp_def
        Pubkey::new_unique(), // cluster
        Pubkey::new_unique(), // pool
        Pubkey::new_unique(), // clock
        Pubkey::new_unique(), // system
        Pubkey::new_unique(), // arcium
        123,
        2,
    );
    assert_eq!(ix.accounts.len(), 12);
}


