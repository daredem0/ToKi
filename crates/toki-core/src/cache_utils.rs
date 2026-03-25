pub fn clone_cached_or_load<V, E>(
    cached: Option<V>,
    load: impl FnOnce() -> Result<V, E>,
    store: impl FnOnce(V),
) -> Result<V, E>
where
    V: Clone,
{
    if let Some(cached) = cached {
        return Ok(cached.clone());
    }

    let value = load()?;
    store(value.clone());
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::clone_cached_or_load;

    #[test]
    fn clone_cached_or_load_returns_cached_value_without_loading() {
        let cached = String::from("cached");
        let mut load_called = false;

        let value = clone_cached_or_load(
            Some(cached.clone()),
            || {
                load_called = true;
                Ok::<String, &'static str>("loaded".to_string())
            },
            |_| {},
        )
        .expect("cached result");

        assert_eq!(value, "cached");
        assert!(!load_called);
    }

    #[test]
    fn clone_cached_or_load_loads_and_stores_on_cache_miss() {
        let mut stored = None;

        let value = clone_cached_or_load(
            None::<String>,
            || Ok::<String, &'static str>("loaded".to_string()),
            |loaded| stored = Some(loaded),
        )
        .expect("loaded result");

        assert_eq!(value, "loaded");
        assert_eq!(stored.as_deref(), Some("loaded"));
    }
}
