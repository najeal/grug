use {
    crate::{Batch, Hash, Storage},
    serde::{de::DeserializeOwned, ser::Serialize},
};

// ------------------------------------ db -------------------------------------

/// The API that a backing database must implement.
///
/// The database should follow [ADR-065](https://github.com/cosmos/cosmos-sdk/blob/main/docs/architecture/adr-065-store-v2.md).
/// That is, it should contain two components:
/// - **state commitment**, a _Merklized_ KV store that stores _hashed_ keys and
///   _hashed_ values;
/// - **state storage**, a _flat_ KV store that stores _raw_ keys and _raw_ values.
///
/// The `state_commitment` and `state_storage` methods should return an _owned_
/// instance of the storage object (see the required `'static` lifetime). This
/// is required by the Wasm runtime. Additionally, storage object should be
/// _read only_. The host should write changes to an in-memory caching layer
/// (e.g. using `cw_db::CacheStore`) and then use the `flush` and `commit`
/// methods to persist them.
///
/// The two mutable methods `flush` and `commit` take an immutable reference
/// of self (`&self`) instead of a mutable one. This is needed for multithreading.
/// For this reason, the implementation should use the [interior mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
/// pattern such as `Arc<RwLock<T>>`.
///
/// We ship three implementations of `Db`:
/// - for use in production nodes, a physical DB based on RocksDB;
/// - for testing, a temporary, in-memory KV store;
/// - for tests that utilize mainnet data ("mainnet forking"), a DB that pulls
///   data from a remote archive node.
pub trait Db {
    type Error: ToString;

    /// Type of the Merkle proof. The DB can choose any Merkle tree scheme.
    type Proof: Serialize + DeserializeOwned;

    /// Return the state commitment as an owned, read-only, `Storage` object.
    /// This should be a _Merklized_ KV store that stores _hashed_ keys and _hashed_ values.
    fn state_commitment(&self) -> impl Storage + Clone + 'static;

    /// Return the state storage as an owned, read-only, `Storage` object.
    /// This should be a _flat_ KV store that stores _raw_ keys and _raw_ values.
    fn state_storage(&self, version: Option<u64>) -> impl Storage + Clone + 'static;

    /// Return the most recent version that has been committed.
    /// `None` if not a single version has been committed.
    fn latest_version(&self) -> Option<u64>;

    /// Return the Merkle root hash at the specified version.
    /// If version is unspecified, return that of the latest committed version.
    /// `None` if the Merkle tree is empty at that version, or if that version
    /// has been pruned (we can't differentiate these two situations).
    fn root_hash(&self, version: Option<u64>) -> Result<Option<Hash>, Self::Error>;

    /// Generate Merkle proof of the given key at the given version.
    /// If version is unspecified, use the latest version.
    /// If the key exists at that version, the returned value should be a Merkle
    /// _membership_ proof; otherwise, it should be a _non-membership_ proof.
    fn prove(&self, key: &[u8], version: Option<u64>) -> Result<Self::Proof, Self::Error>;

    /// Accept a batch ops (an op is either a DB insertion or a deletion), keep
    /// them in the memory, but do not persist to disk yet; also, increment the
    /// version.
    ///
    /// This is typically invoked in the ABCI `FinalizeBlock` call.
    fn flush_but_not_commit(&self, batch: Batch) -> Result<(u64, Option<Hash>), Self::Error>;

    /// Persist pending data added in the `flush` method to disk.
    ///
    /// This is typically invoked in the ABCI `Commit` call.
    fn commit(&self) -> Result<(), Self::Error>;

    /// Flush and commit in one go.
    ///
    /// This is typically only invoked in the ABCI `InitChain` call.
    fn flush_and_commit(&self, batch: Batch) -> Result<(u64, Option<Hash>), Self::Error> {
        let (new_version, root_hash) = self.flush_but_not_commit(batch)?;
        self.commit()?;
        Ok((new_version, root_hash))
    }
}

// ------------------------------------ vm -------------------------------------

pub trait Vm {
    type Error: ToString;
}