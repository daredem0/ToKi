use crate::entity::EntityId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparseComponentMap<T> {
    entries: HashMap<EntityId, T>,
}

impl<T> Default for SparseComponentMap<T> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<T> SparseComponentMap<T> {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, id: EntityId) -> bool {
        self.entries.contains_key(&id)
    }

    pub fn get(&self, id: EntityId) -> Option<&T> {
        self.entries.get(&id)
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut T> {
        self.entries.get_mut(&id)
    }

    pub fn insert(&mut self, id: EntityId, value: T) -> Option<T> {
        self.entries.insert(id, value)
    }

    pub fn remove(&mut self, id: EntityId) -> Option<T> {
        self.entries.remove(&id)
    }

    pub fn ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entries.keys().copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &T)> + '_ {
        self.entries.iter().map(|(id, value)| (*id, value))
    }
}

impl<T> Serialize for SparseComponentMap<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.entries.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for SparseComponentMap<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = HashMap::deserialize(deserializer)?;
        Ok(Self { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::SparseComponentMap;

    #[test]
    fn sparse_component_map_round_trip_operations() {
        let mut map = SparseComponentMap::default();
        assert!(map.is_empty());
        assert!(!map.contains(1));

        assert_eq!(map.insert(1, "a"), None);
        assert_eq!(map.get(1), Some(&"a"));
        assert!(map.contains(1));
        assert_eq!(map.insert(1, "b"), Some("a"));
        assert_eq!(map.get(1), Some(&"b"));
        assert_eq!(map.ids().collect::<Vec<_>>(), vec![1]);
        assert_eq!(map.remove(1), Some("b"));
        assert!(map.is_empty());
    }

    #[test]
    fn sparse_component_map_serialization_round_trip() {
        let mut map = SparseComponentMap::default();
        map.insert(2, 7_u32);
        let json = serde_json::to_string(&map).expect("map should serialize");
        let restored: SparseComponentMap<u32> =
            serde_json::from_str(&json).expect("map should deserialize");
        assert_eq!(restored.get(2), Some(&7));
    }
}
