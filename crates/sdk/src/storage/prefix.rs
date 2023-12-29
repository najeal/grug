use {
    super::{concat, extend_one_byte, increment_last_byte, nested_namespaces_with_key, trim},
    crate::{from_json, MapKey, Order, RawKey, Storage},
    serde::de::DeserializeOwned,
    std::ops::Bound,
};

pub struct Prefix {
    prefix: Vec<u8>,
}

impl Prefix {
    pub fn new(namespace: &[u8], prefixes: &[RawKey]) -> Self {
        Self {
            prefix: nested_namespaces_with_key(Some(namespace), prefixes, None),
        }
    }

    pub fn range<'a, K, T>(
        &self,
        store: &'a dyn Storage,
        min:   Bound<&K>,
        max:   Bound<&K>,
        order: Order,
    ) -> Box<dyn Iterator<Item = anyhow::Result<(K, T)>> + 'a>
    where
        K: MapKey,
        T: DeserializeOwned,
    {
        // compute start and end bounds
        // note that the store considers the start bounds as inclusive, and end
        // bound as exclusive (see the Storage trait)
        let min = match min {
            Bound::Unbounded => self.prefix.to_vec(),
            Bound::Included(k) => concat(&self.prefix, &k.serialize()),
            Bound::Excluded(k) => extend_one_byte(concat(&self.prefix, &k.serialize())),
        };
        let max = match max {
            Bound::Unbounded => increment_last_byte(self.prefix.to_vec()),
            Bound::Included(k) => extend_one_byte(concat(&self.prefix, &k.serialize())),
            Bound::Excluded(k) => concat(&self.prefix, &k.serialize()),
        };

        // need to make a clone of self.prefix and move it into the closure,
        // so that the iterator can live longer than &self.
        let prefix = self.prefix.clone();
        let iter = store.scan(Some(&min), Some(&max), order).map(move |(k, v)| {
            debug_assert_eq!(&k[0..prefix.len()], prefix, "Prefix mispatch");
            let key_bytes = trim(&prefix, &k);
            let key = K::deserialize(&key_bytes)?;
            let data = from_json(&v)?;
            Ok((key, data))
        });

        Box::new(iter)
    }

    pub fn clear(&self, _store: &mut dyn Storage) {
        todo!()
    }
}