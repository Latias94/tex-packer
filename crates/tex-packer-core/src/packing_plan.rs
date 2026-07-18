use std::collections::HashMap;

use image::RgbaImage;
use xxhash_rust::xxh3::Xxh3;

use crate::preparation::PreparedItem;

pub(crate) struct PlacementGroup {
    canonical_item_index: usize,
    alias_item_indices: Vec<usize>,
}

impl PlacementGroup {
    pub(crate) fn canonical_item_index(&self) -> usize {
        self.canonical_item_index
    }

    pub(crate) fn alias_item_indices(&self) -> &[usize] {
        &self.alias_item_indices
    }

    pub(crate) fn logical_item_count(&self) -> usize {
        1 + self.alias_item_indices.len()
    }
}

pub(crate) struct PackingPlan {
    groups: Vec<PlacementGroup>,
}

impl PackingPlan {
    pub(crate) fn one_per_item(item_count: usize) -> Self {
        Self {
            groups: (0..item_count)
                .map(|canonical_item_index| PlacementGroup {
                    canonical_item_index,
                    alias_item_indices: Vec::new(),
                })
                .collect(),
        }
    }

    pub(crate) fn deduplicated(prepared: &[PreparedItem<RgbaImage>]) -> Self {
        let mut groups: Vec<PlacementGroup> = Vec::new();
        let mut buckets: HashMap<ContentFingerprint, Vec<usize>> = HashMap::new();

        for item_index in 0..prepared.len() {
            let fingerprint = content_fingerprint(&prepared[item_index]);
            let matching_group = buckets.get(&fingerprint).and_then(|group_indices| {
                group_indices.iter().copied().find(|&group_index| {
                    let canonical_index = groups[group_index].canonical_item_index;
                    same_content(&prepared[canonical_index], &prepared[item_index])
                })
            });

            if let Some(group_index) = matching_group {
                groups[group_index].alias_item_indices.push(item_index);
            } else {
                let group_index = groups.len();
                groups.push(PlacementGroup {
                    canonical_item_index: item_index,
                    alias_item_indices: Vec::new(),
                });
                buckets.entry(fingerprint).or_default().push(group_index);
            }
        }

        Self { groups }
    }

    pub(crate) fn len(&self) -> usize {
        self.groups.len()
    }

    pub(crate) fn group(&self, index: usize) -> &PlacementGroup {
        &self.groups[index]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ContentFingerprint {
    width: u32,
    height: u32,
    hash: u64,
}

fn content_fingerprint(item: &PreparedItem<RgbaImage>) -> ContentFingerprint {
    let mut hasher = Xxh3::new();
    hasher.update(&item.source.w.to_le_bytes());
    hasher.update(&item.source.h.to_le_bytes());
    for row in 0..item.source.h {
        hasher.update(content_row(item, row));
    }

    ContentFingerprint {
        width: item.source.w,
        height: item.source.h,
        hash: hasher.digest(),
    }
}

fn same_content(first: &PreparedItem<RgbaImage>, second: &PreparedItem<RgbaImage>) -> bool {
    first.source.w == second.source.w
        && first.source.h == second.source.h
        && (0..first.source.h).all(|row| content_row(first, row) == content_row(second, row))
}

fn content_row(item: &PreparedItem<RgbaImage>, row: u32) -> &[u8] {
    let image_width = item.payload.width() as usize;
    let y = item.source.y as usize + row as usize;
    let start = (y * image_width + item.source.x as usize) * 4;
    let end = start + item.source.w as usize * 4;
    &item.payload.as_raw()[start..end]
}
