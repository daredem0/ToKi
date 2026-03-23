#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrderedDrawBatch<K> {
    pub(crate) key: K,
    pub(crate) start: usize,
    pub(crate) count: usize,
}

pub(crate) fn append_ordered_draw_batch<K: PartialEq>(
    batches: &mut Vec<OrderedDrawBatch<K>>,
    key: K,
    start: usize,
) {
    if let Some(last) = batches.last_mut() {
        if last.key == key && last.start + last.count == start {
            last.count += 1;
            return;
        }
    }

    batches.push(OrderedDrawBatch {
        key,
        start,
        count: 1,
    });
}

#[cfg(test)]
mod tests {
    use super::{append_ordered_draw_batch, OrderedDrawBatch};

    #[test]
    fn append_ordered_draw_batch_merges_only_consecutive_ranges_of_same_key() {
        let mut batches = Vec::new();

        append_ordered_draw_batch(&mut batches, "a", 0);
        append_ordered_draw_batch(&mut batches, "a", 1);
        append_ordered_draw_batch(&mut batches, "b", 0);
        append_ordered_draw_batch(&mut batches, "a", 2);

        assert_eq!(
            batches,
            vec![
                OrderedDrawBatch {
                    key: "a",
                    start: 0,
                    count: 2,
                },
                OrderedDrawBatch {
                    key: "b",
                    start: 0,
                    count: 1,
                },
                OrderedDrawBatch {
                    key: "a",
                    start: 2,
                    count: 1,
                },
            ]
        );
    }
}
