pub mod ext4;
pub mod exfat;
pub mod vfat;
pub mod volume_id;

/* Tags
TYPE:           filesystem type
SEC_TYPE:       Secondary filesystem type
LABEL:          fs label
LABEL_RAW:      Raw fs label
UUID:           fs uuid
UUID_RAW:       raw uuid
UUID_SUB:       Sub uuid
LOG_UUID:       external log uuid
LOG_UUID_RAW:       external log uuid
EXT_JOURNAL:    external journal uuid
USAGE:          usage string 
VERSION:        fs version
SBMAGIC:        superblock magic string
SBMAGIC_OFFSET: magic offset
FSSIZE:         size of filesystem
FSLASTBLOCK:    offset of last sector in superblock   
FSBLOCKSIZE:    fs block size
BLOCK_SIZE:     block size of phyical disk
*/

pub trait ProbeFilesystem {
    fn probe_is(&self) -> Result<(), Box<dyn std::error::Error>>;
}