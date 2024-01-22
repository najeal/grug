//! How to run this example:
//!
//! $ just optimize
//! $ cargo run -p cw-app --example bank

use {
    cfg_if::cfg_if,
    cw_account::{sign_bytes, PubKey},
    cw_app::App,
    cw_bank::Balance,
    cw_crypto::Identity256,
    cw_db::MockStorage,
    cw_std::{
        hash, to_json, Addr, Binary, BlockInfo, Coin, Coins, Config, GenesisState, Hash, Message,
        QueryRequest, Storage, Tx, Uint128,
    },
    k256::ecdsa::{signature::DigestSigner, Signature, SigningKey, VerifyingKey},
    lazy_static::lazy_static,
    rand::{rngs::StdRng, SeedableRng},
    std::{collections::BTreeMap, env, fs::File, io::Read, path::PathBuf},
};

lazy_static! {
    // chain ID for the purpose of this example
    static ref CHAIN_ID: &'static str = "dev-1";

    // during genesis, we use an all-zero address as the deployer
    static ref DEPLOYER: Addr = Addr::mock(0);

    // salt for instantiating the bank contract
    static ref BANK_SALT: Binary = b"bank".to_vec().into();

    // salt for insantiating user accounts
    static ref ACCT_SALT: fn(usize) -> Binary = |idx: usize| Binary::from(format!("account-{idx}").into_bytes());
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    println!("🤖 Creating app");
    let mut app = App::new(MockStorage::new());

    println!("🤖 Reading wasm byte codes from files");
    let account_wasm = read_wasm_byte_code("cw_account")?;
    let bank_wasm = read_wasm_byte_code("cw_bank")?;

    println!("🤖 Computing bank contract address");
    let bank_addr = Addr::compute(&DEPLOYER, &bank_wasm.hash, &BANK_SALT);

    println!("🤖 Generate random accounts");
    let accounts = make_random_accounts(6, &account_wasm.hash);

    println!("🤖 Initialize chain");
    app.init_chain(make_genesis_state(&accounts, &account_wasm, &bank_wasm, &bank_addr)?)?;

    println!("🤖 Making transfers");
    let block = make_block_info(1, 10000);
    let txs = vec![
        make_transfer_tx(&accounts, 0, [
            TestTransfer {
                to:     3,
                denom:  "uatom",
                amount: 75,
            },
            TestTransfer {
                to:     4,
                denom:  "uosmo",
                amount: 420,
            },
        ])?,
        make_transfer_tx(&accounts, 1, [
            TestTransfer {
                to:     2,
                denom:  "uatom",
                amount: 50,
            },
        ])?,
        make_transfer_tx(&accounts, 2, [
            TestTransfer {
                to:     0,
                denom:  "uatom",
                amount: 69,
            },
        ])?,
        make_transfer_tx(&accounts, 4, [
            TestTransfer {
                to:     5,
                denom:  "uosmo",
                amount: 64,
            },
        ])?,
    ];
    app.finalize_block(block, txs)?;
    app.commit()?;

    println!("🤖 Querying balances after transfers");
    query_all_balances(&mut app, &accounts)?;

    println!("✅ Done!");

    Ok(())
}

struct TestCode {
    pub hash:      Hash,
    pub byte_code: Binary,
}

struct TestAccount {
    pub addr: Addr,
    pub sk:   SigningKey,
    pub vk:   VerifyingKey,
}

struct TestTransfer {
    to:     usize,
    denom:  &'static str,
    amount: u128,
}

fn read_wasm_byte_code(name: &str) -> anyhow::Result<TestCode> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let wasm_file_path = manifest_dir.join({
        cfg_if! {
            if #[cfg(target_arch = "aarch64")] {
                format!("../../artifacts/{name}-aarch64.wasm")
            } else {
                format!("../../artifacts/{name}.wasm")
            }
        }
    });
    let mut wasm_file = File::open(wasm_file_path)?;
    let mut byte_code = Vec::new();
    wasm_file.read_to_end(&mut byte_code)?;

    Ok(TestCode {
        hash:      hash(&byte_code),
        byte_code: byte_code.into(),
    })
}

fn make_random_accounts(count: usize, code_hash: &Hash) -> Vec<TestAccount> {
    let mut rng = StdRng::seed_from_u64(42);
    let mut accts = <Vec<TestAccount>>::with_capacity(count);
    for idx in 0..count {
        let sk = SigningKey::random(&mut rng);
        let vk = VerifyingKey::from(&sk);
        let addr = Addr::compute(&DEPLOYER, code_hash, &ACCT_SALT(idx));
        accts.push(TestAccount { addr, sk, vk });
    }
    accts
}

fn make_initial_balances(accounts: &[TestAccount]) -> Vec<Balance> {
    vec![
        Balance {
            address: accounts[0].addr.clone(),
            coins: Coins::from_vec_unchecked(vec![
                Coin {
                    denom: "uatom".into(),
                    amount: Uint128::new(100),
                },
                Coin {
                    denom: "uosmo".into(),
                    amount: Uint128::new(888),
                },
            ]),
        },
        Balance {
            address: accounts[1].addr.clone(),
            coins: Coins::from(Coin {
                denom: "uatom".into(),
                amount: Uint128::new(50),
            }),
        },
        Balance {
            address: accounts[2].addr.clone(),
            coins: Coins::from(Coin {
                denom: "uatom".into(),
                amount: Uint128::new(123),
            }),
        },
    ]
}

fn make_genesis_state(
    accounts:     &[TestAccount],
    account_wasm: &TestCode,
    bank_wasm:    &TestCode,
    bank_addr:    &Addr,
) -> anyhow::Result<GenesisState> {
    // upload codes and instantiate bank contract
    let mut gen_state = GenesisState {
        chain_id: CHAIN_ID.to_string(),
        config: Config {
            bank: bank_addr.clone(),
        },
        msgs: vec![
            Message::StoreCode {
                wasm_byte_code: account_wasm.byte_code.clone(),
            },
            Message::StoreCode {
                wasm_byte_code: bank_wasm.byte_code.clone(),
            },
            Message::Instantiate {
                code_hash: bank_wasm.hash.clone(),
                msg: to_json(&cw_bank::InstantiateMsg {
                    initial_balances: make_initial_balances(accounts),
                })?,
                salt:  BANK_SALT.clone(),
                funds: Coins::empty(),
                admin: None,
            },
        ]
    };

    // instantiate user accounts
    for (idx, acct) in accounts.iter().enumerate() {
        gen_state.msgs.push(Message::Instantiate {
            code_hash: account_wasm.hash.clone(),
            msg: to_json(&cw_account::InstantiateMsg {
                pubkey: PubKey::Secp256k1(acct.vk.to_sec1_bytes().to_vec().into()),
            })?,
            salt: ACCT_SALT(idx),
            funds: Coins::empty(),
            admin: Some(acct.addr.clone()),
        });
    }

    Ok(gen_state)
}

fn make_transfer_tx<const N: usize>(
    accounts:  &[TestAccount],
    from:      usize,
    transfers: [TestTransfer; N],
) -> anyhow::Result<Tx> {
    let mut msgs = vec![];
    for TestTransfer { to, denom, amount } in transfers {
        msgs.push(Message::Transfer {
            to: accounts[to].addr.clone(),
            coins: Coins::from(Coin {
                denom:  denom.to_string(),
                amount: Uint128::new(amount),
            }),
        });
    }

    // for the purpose of this example, we assume sequence number is zero
    let sign_bytes = Identity256::from_bytes(&sign_bytes(
        &msgs,
        &accounts[from].addr,
        &CHAIN_ID,
        0,
    )?)?;
    let signature: Signature = accounts[from].sk.sign_digest(sign_bytes);

    Ok(Tx {
        sender:     accounts[from].addr.clone(),
        credential: signature.to_vec().into(),
        msgs,
    })
}

fn make_block_info(height: u64, timestamp: u64) -> BlockInfo {
    BlockInfo {
        chain_id: "dev-1".into(),
        height,
        timestamp,
    }
}

fn query_all_balances<S>(app: &mut App<S>, accounts: &[TestAccount]) -> anyhow::Result<()>
where
    S: Storage + 'static,
{
    let mut resps = BTreeMap::new();
    for acct in accounts {
        let balances = app
            .query(QueryRequest::Balances {
                address: acct.addr.clone(),
                start_after: None,
                limit: None,
            })?
            .as_balances();

        if !balances.is_empty() {
            resps.insert(acct.addr.clone(), balances);
        }
    }

    println!("{}", serde_json::to_string_pretty(&resps)?);

    Ok(())
}
