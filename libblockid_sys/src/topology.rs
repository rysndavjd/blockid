#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum AlignmentOffset {
    Misaligned,
    Offset(u64),
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyTag {
    LogicalSectorSize(u64),
    PhysicalSectorSize(u64),
    MinimumIoSize(u64),
    OptimalIoSize(u64),
    AlignmentOffset(AlignmentOffset),
}

#[derive(Debug)]
pub struct TopologyInfo {
    tags: Vec<TopologyTag>,
}

impl TopologyInfo {
    pub(crate) fn new() -> TopologyInfo {
        TopologyInfo { tags: Vec::new() }
    }

    pub fn inner(&self) -> &Vec<TopologyTag> {
        &self.tags
    }

    pub fn into_inner(self) -> Vec<TopologyTag> {
        self.tags
    }

    pub fn set(&mut self, tag: TopologyTag) {
        self.tags.push(tag);
    }

    pub fn logical_sector_size(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            TopologyTag::LogicalSectorSize(t) => Some(*t),
            _ => None,
        })
    }

    pub fn physical_sector_size(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            TopologyTag::PhysicalSectorSize(t) => Some(*t),
            _ => None,
        })
    }

    pub fn minimum_io_size(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            TopologyTag::MinimumIoSize(t) => Some(*t),
            _ => None,
        })
    }

    pub fn optimal_io_size(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            TopologyTag::OptimalIoSize(t) => Some(*t),
            _ => None,
        })
    }

    pub fn alignment_offset(&self) -> Option<AlignmentOffset> {
        self.tags.iter().find_map(|t| match t {
            TopologyTag::AlignmentOffset(t) => Some(*t),
            _ => None,
        })
    }
}
