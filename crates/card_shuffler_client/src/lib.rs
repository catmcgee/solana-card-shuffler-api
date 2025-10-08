use anchor_lang::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::hash::hashv;

declare_id!("6GvHYdjrQSFPb6TBSx49w5KUb37a3ZdyAwv9D2sHMGWD");

pub const SHUFFLE_DEAL_PROGRAM_ID: Pubkey = pubkey!("9heUwYsyauSVrMwv3kULC93Ck35WxWyG6EDqcrfJ5me");
pub const PUBLIC_REVEAL_PROGRAM_ID: Pubkey = pubkey!("4PAak7n6BLNFsYY69wRdWLE2Cs827YzQgd7CMat6XvzS");
pub const REWRAP_PROGRAM_ID: Pubkey = pubkey!("pS6VM9iBY2xycNJ5gZ5hJxMT3aejVXWoVbggCVQ8ZJ2");

#[error_code]
pub enum ClientError {
    #[msg("Indices must not be empty")] 
    EmptyIndices,
}

pub type ClientResult<T> = core::result::Result<T, ClientError>;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct DeckComputationContext {
    pub computation_offset: u64,
}

pub fn set_deck_computation_ix(payer: Pubkey, computation_offset: u64) -> Instruction {
    let accounts = vec![AccountMeta::new(payer, true)];
    let data = pack_ix_data("set_deck_computation", &SetDeckComputationData { computation_offset });
    Instruction { program_id: crate::ID, accounts, data }
}

pub fn store_encrypted_hole_cards_ix(
    payer: Pubkey,
    owner: Pubkey,
    hand_ciphertext: Vec<u8>,
) -> Instruction {
    let accounts = vec![AccountMeta::new(payer, true)];
    let data = pack_ix_data("store_encrypted_hole_cards", &StoreEncryptedHoleCardsData { owner, hand_ciphertext });
    Instruction { program_id: crate::ID, accounts, data }
}

pub fn reveal_community_cards_ix(
    payer: Pubkey,
    indices: Vec<u8>,
) -> ClientResult<Instruction> {
    if indices.is_empty() {
        return Err(ClientError::EmptyIndices);
    }
    let accounts = vec![AccountMeta::new(payer, true)];
    let data = pack_ix_data("reveal_community_cards", &RevealCommunityCardsData { indices });
    Ok(Instruction { program_id: crate::ID, accounts, data })
}

#[derive(AnchorSerialize, AnchorDeserialize)]
struct SetDeckComputationData { computation_offset: u64 }

#[derive(AnchorSerialize, AnchorDeserialize)]
struct StoreEncryptedHoleCardsData { owner: Pubkey, hand_ciphertext: Vec<u8> }

#[derive(AnchorSerialize, AnchorDeserialize)]
struct RevealCommunityCardsData { indices: Vec<u8> }

fn anchor_sighash(ix_name: &str) -> [u8; 8] {
    let h = hashv(&[b"global", ix_name.as_bytes()]).to_bytes();
    let mut out = [0u8; 8];
    out.copy_from_slice(&h[..8]);
    out
}

fn pack_ix_data<T: AnchorSerialize>(ix_name: &str, args: &T) -> Vec<u8> {
    let mut data = anchor_sighash(ix_name).to_vec();
    let mut args_bytes = args.try_to_vec().expect("serialize args");
    data.append(&mut args_bytes);
    data
}

pub mod arcium_flows {
    use super::*;

    pub fn init_shuffle_and_deal_comp_def_ix(
        comp_def_account: Pubkey,
        payer: Pubkey,
        mxe_account: Pubkey,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new(comp_def_account, false),
            AccountMeta::new(payer, true),
            AccountMeta::new(mxe_account, false),
        ];
        let data = pack_ix_data("init_shuffle_and_deal_comp_def", &());
        Instruction { program_id: SHUFFLE_DEAL_PROGRAM_ID, accounts, data }
    }

    // shuffle_deal: queue shuffle_and_deal
    #[derive(AnchorSerialize, AnchorDeserialize)]
    struct ShuffleAndDealArgs { computation_offset: u64, cards_per_player: u8 }

    pub fn shuffle_and_deal_ix(
        payer: Pubkey,
        sign_pda_account: Pubkey,
        mxe_account: Pubkey,
        mempool_account: Pubkey,
        executing_pool: Pubkey,
        computation_account: Pubkey,
        comp_def_account: Pubkey,
        cluster_account: Pubkey,
        pool_account: Pubkey,
        clock_account: Pubkey,
        system_program: Pubkey,
        arcium_program: Pubkey,
        computation_offset: u64,
        cards_per_player: u8,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(sign_pda_account, false),
            AccountMeta::new_readonly(mxe_account, false),
            AccountMeta::new(mempool_account, false),
            AccountMeta::new(executing_pool, false),
            AccountMeta::new(computation_account, false),
            AccountMeta::new_readonly(comp_def_account, false),
            AccountMeta::new(cluster_account, false),
            AccountMeta::new(pool_account, false),
            AccountMeta::new_readonly(clock_account, false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(arcium_program, false),
        ];
        let data = pack_ix_data(
            "shuffle_and_deal",
            &ShuffleAndDealArgs { computation_offset, cards_per_player },
        );
        Instruction { program_id: SHUFFLE_DEAL_PROGRAM_ID, accounts, data }
    }

    // public_reveal: init comp defs
    pub fn init_reveal_card_comp_def_ix(comp_def_account: Pubkey, payer: Pubkey, mxe_account: Pubkey) -> Instruction {
        let accounts = vec![
            AccountMeta::new(comp_def_account, false),
            AccountMeta::new(payer, true),
            AccountMeta::new(mxe_account, false),
        ];
        let data = pack_ix_data("init_reveal_card_comp_def", &());
        Instruction { program_id: PUBLIC_REVEAL_PROGRAM_ID, accounts, data }
    }

    pub fn init_reveal_hand_comp_def_ix(comp_def_account: Pubkey, payer: Pubkey, mxe_account: Pubkey) -> Instruction {
        let accounts = vec![
            AccountMeta::new(comp_def_account, false),
            AccountMeta::new(payer, true),
            AccountMeta::new(mxe_account, false),
        ];
        let data = pack_ix_data("init_reveal_hand_comp_def", &());
        Instruction { program_id: PUBLIC_REVEAL_PROGRAM_ID, accounts, data }
    }

    // public_reveal: queue reveal_card
    #[derive(AnchorSerialize, AnchorDeserialize)]
    struct RevealCardArgs { computation_offset: u64, card_index: u8 }

    pub fn reveal_card_ix(
        payer: Pubkey,
        sign_pda_account: Pubkey,
        mxe_account: Pubkey,
        mempool_account: Pubkey,
        executing_pool: Pubkey,
        computation_account: Pubkey,
        comp_def_account: Pubkey,
        cluster_account: Pubkey,
        pool_account: Pubkey,
        clock_account: Pubkey,
        system_program: Pubkey,
        arcium_program: Pubkey,
        computation_offset: u64,
        card_index: u8,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(sign_pda_account, false),
            AccountMeta::new_readonly(mxe_account, false),
            AccountMeta::new(mempool_account, false),
            AccountMeta::new(executing_pool, false),
            AccountMeta::new(computation_account, false),
            AccountMeta::new_readonly(comp_def_account, false),
            AccountMeta::new(cluster_account, false),
            AccountMeta::new(pool_account, false),
            AccountMeta::new_readonly(clock_account, false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(arcium_program, false),
        ];
        let data = pack_ix_data("reveal_card", &RevealCardArgs { computation_offset, card_index });
        Instruction { program_id: PUBLIC_REVEAL_PROGRAM_ID, accounts, data }
    }

    // public_reveal: queue reveal_hand
    #[derive(AnchorSerialize, AnchorDeserialize)]
    struct RevealHandArgs { computation_offset: u64 }

    pub fn reveal_hand_ix(
        payer: Pubkey,
        sign_pda_account: Pubkey,
        mxe_account: Pubkey,
        mempool_account: Pubkey,
        executing_pool: Pubkey,
        computation_account: Pubkey,
        comp_def_account: Pubkey,
        cluster_account: Pubkey,
        pool_account: Pubkey,
        clock_account: Pubkey,
        system_program: Pubkey,
        arcium_program: Pubkey,
        computation_offset: u64,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(sign_pda_account, false),
            AccountMeta::new_readonly(mxe_account, false),
            AccountMeta::new(mempool_account, false),
            AccountMeta::new(executing_pool, false),
            AccountMeta::new(computation_account, false),
            AccountMeta::new_readonly(comp_def_account, false),
            AccountMeta::new(cluster_account, false),
            AccountMeta::new(pool_account, false),
            AccountMeta::new_readonly(clock_account, false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(arcium_program, false),
        ];
        let data = pack_ix_data("reveal_hand", &RevealHandArgs { computation_offset });
        Instruction { program_id: PUBLIC_REVEAL_PROGRAM_ID, accounts, data }
    }

    // rewrap: transfer_hand comp def
    pub fn init_transfer_hand_comp_def_ix(comp_def_account: Pubkey, payer: Pubkey, mxe_account: Pubkey) -> Instruction {
        let accounts = vec![
            AccountMeta::new(comp_def_account, false),
            AccountMeta::new(payer, true),
            AccountMeta::new(mxe_account, false),
        ];
        let data = pack_ix_data("init_transfer_hand_comp_def", &());
        Instruction { program_id: REWRAP_PROGRAM_ID, accounts, data }
    }

    // rewrap: queue transfer_hand
    #[derive(AnchorSerialize, AnchorDeserialize)]
    struct TransferHandArgs { computation_offset: u64 }

    pub fn transfer_hand_ix(
        payer: Pubkey,
        sign_pda_account: Pubkey,
        mxe_account: Pubkey,
        mempool_account: Pubkey,
        executing_pool: Pubkey,
        computation_account: Pubkey,
        comp_def_account: Pubkey,
        cluster_account: Pubkey,
        pool_account: Pubkey,
        clock_account: Pubkey,
        system_program: Pubkey,
        arcium_program: Pubkey,
        computation_offset: u64,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(sign_pda_account, false),
            AccountMeta::new_readonly(mxe_account, false),
            AccountMeta::new(mempool_account, false),
            AccountMeta::new(executing_pool, false),
            AccountMeta::new(computation_account, false),
            AccountMeta::new_readonly(comp_def_account, false),
            AccountMeta::new(cluster_account, false),
            AccountMeta::new(pool_account, false),
            AccountMeta::new_readonly(clock_account, false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(arcium_program, false),
        ];
        let data = pack_ix_data("transfer_hand", &TransferHandArgs { computation_offset });
        Instruction { program_id: REWRAP_PROGRAM_ID, accounts, data }
    }
}


