//! Stable validation/parse codes (append-only: never rename or reuse, D-031) with English messages.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Code {
    TooShort,
    BadMagic,
    EncryptedUnsupported,
    TooManyEntries,
    NamesTooLarge,
    TableOutOfBounds,
    NameOffsetOob,
    NameUnterminated,
    NameInvalid,
    DirRangeOob,
    TableCycle,
    SharedChild,
    DepthExceeded,
    DataOob,
    DataInTable,
    NoRoot,
    Io,
    // ---- M3 (append-only, D-031) ----
    DuplicateName,
    DataOverlap,
    OrphanEntries,
    EditPathNotFound,
    EditNotAFolder,
    EditNameExists,
    EditRootProtected,
    EditTargetIsFolder,
    EditResourceUnsupported,
    EditNoChange,
    EditFileTooLarge,
    ArchiveTooLarge,
    NameOffsetTooLarge,
    BlobMissing,
    BlobLengthMismatch,
    RebuildVerifyFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A code plus a specific detail (for example the path involved).
#[derive(Debug, Clone)]
pub struct Fail {
    pub code: Code,
    pub detail: String,
}
impl Fail {
    pub fn new(code: Code, detail: impl Into<String>) -> Fail {
        Fail { code, detail: detail.into() }
    }
}
impl From<Code> for Fail {
    fn from(code: Code) -> Fail {
        Fail { code, detail: String::new() }
    }
}

impl Code {
    /// Stable machine key.
    pub fn as_str(&self) -> &'static str {
        match self {
            Code::TooShort => "RPF_TOO_SHORT",
            Code::BadMagic => "RPF_HEADER_BAD_MAGIC",
            Code::EncryptedUnsupported => "RPF_ENCRYPTED_UNSUPPORTED",
            Code::TooManyEntries => "RPF_TOO_MANY_ENTRIES",
            Code::NamesTooLarge => "RPF_NAMES_TOO_LARGE",
            Code::TableOutOfBounds => "RPF_TABLE_OUT_OF_BOUNDS",
            Code::NameOffsetOob => "NAME_OFFSET_OOB",
            Code::NameUnterminated => "NAME_UNTERMINATED",
            Code::NameInvalid => "NAME_INVALID",
            Code::DirRangeOob => "RPF_DIR_RANGE_OOB",
            Code::TableCycle => "RPF_TABLE_CYCLE",
            Code::SharedChild => "RPF_SHARED_CHILD",
            Code::DepthExceeded => "RPF_DEPTH_EXCEEDED",
            Code::DataOob => "DATA_OUT_OF_BOUNDS",
            Code::DataInTable => "DATA_OVERLAPS_TABLE",
            Code::NoRoot => "RPF_NO_ROOT",
            Code::Io => "IO_ERROR",
            Code::DuplicateName => "NAME_DUPLICATE",
            Code::DataOverlap => "DATA_OVERLAP",
            Code::OrphanEntries => "ORPHAN_ENTRIES",
            Code::EditPathNotFound => "EDIT_PATH_NOT_FOUND",
            Code::EditNotAFolder => "EDIT_NOT_A_FOLDER",
            Code::EditNameExists => "EDIT_NAME_EXISTS",
            Code::EditRootProtected => "EDIT_ROOT_PROTECTED",
            Code::EditTargetIsFolder => "EDIT_TARGET_IS_FOLDER",
            Code::EditResourceUnsupported => "EDIT_RESOURCE_UNSUPPORTED",
            Code::EditNoChange => "EDIT_NO_CHANGE",
            Code::EditFileTooLarge => "EDIT_FILE_TOO_LARGE",
            Code::ArchiveTooLarge => "ARCHIVE_TOO_LARGE",
            Code::NameOffsetTooLarge => "NAME_OFFSET_TOO_LARGE",
            Code::BlobMissing => "EDIT_CONTENT_MISSING",
            Code::BlobLengthMismatch => "EDIT_CONTENT_LENGTH_MISMATCH",
            Code::RebuildVerifyFailed => "REBUILD_VERIFY_FAILED",
        }
    }

    /// What went wrong, in plain US English.
    pub fn message(&self) -> &'static str {
        match self {
            Code::TooShort => "The file is too small to be an RPF archive.",
            Code::BadMagic => "The file does not start with the RPF7 header, so it is not an RPF7 archive.",
            Code::EncryptedUnsupported => "This archive is encrypted. Encrypted archives are detected but cannot be opened in this version.",
            Code::TooManyEntries => "The archive declares more entries than the supported maximum.",
            Code::NamesTooLarge => "The archive's name table is larger than the supported maximum.",
            Code::TableOutOfBounds => "The archive's entry table extends past the end of the file, so the file is truncated or damaged.",
            Code::NameOffsetOob => "An entry points to a name outside the name table.",
            Code::NameUnterminated => "An entry name is not terminated inside the name table.",
            Code::NameInvalid => "An entry name is not allowed (empty, a path component such as \"..\", invalid text, or it contains a path separator or control character).",
            Code::DirRangeOob => "A folder lists children that are outside the entry table.",
            Code::TableCycle => "The folder structure contains a loop.",
            Code::SharedChild => "Two folders claim the same child entry, which is not allowed.",
            Code::DepthExceeded => "The folder structure is nested deeper than the supported maximum.",
            Code::DataOob => "An entry's data extends past the end of the file.",
            Code::DataInTable => "An entry's data overlaps the archive's own header or tables.",
            Code::NoRoot => "The archive has no valid root folder.",
            Code::Io => "The archive could not be read.",
            Code::DuplicateName => "Two entries in the same folder have the same name (names are compared without regard to case).",
            Code::DataOverlap => "Two entries claim overlapping data in the archive file.",
            Code::OrphanEntries => "Some entries are not reachable from the root folder. They are ignored and dropped when the archive is saved.",
            Code::EditPathNotFound => "The path does not exist in the archive.",
            Code::EditNotAFolder => "The target must be a folder, but it is a file.",
            Code::EditNameExists => "An entry with that name already exists in the folder (names are compared without regard to case).",
            Code::EditRootProtected => "The root folder cannot be renamed or deleted.",
            Code::EditTargetIsFolder => "This operation works on files, but the target is a folder.",
            Code::EditResourceUnsupported => "Resource entries (RSC7) cannot have their contents replaced in this version. They can still be renamed, deleted or copied unchanged.",
            Code::EditNoChange => "The change would not alter anything.",
            Code::EditFileTooLarge => "The file is larger than a single archive entry can hold (4 GiB minus one byte).",
            Code::ArchiveTooLarge => "The rebuilt archive would be larger than the RPF7 format can address (about 4 GiB).",
            Code::NameOffsetTooLarge => "There are too many distinct file names: a file name must start within the first 64 KiB of the name table.",
            Code::BlobMissing => "The uploaded content for a change is no longer available.",
            Code::BlobLengthMismatch => "The uploaded content for a change does not have the size that was recorded.",
            Code::RebuildVerifyFailed => "The rebuilt archive did not match what was expected, so it was not published.",
        }
    }

    /// What the user can do next.
    pub fn next_step(&self) -> &'static str {
        match self {
            Code::EncryptedUnsupported => "Decrypt the archive with a tool you trust, then upload it again.",
            Code::TooManyEntries | Code::NamesTooLarge => "Split the archive into smaller archives and upload them separately.",
            Code::Io => "Try again. If the problem continues, upload the file again.",
            Code::NameOffsetTooLarge => "Delete files you do not need, or use fewer distinct file names, then save again.",
            Code::ArchiveTooLarge | Code::EditFileTooLarge => "Delete or shrink files, or split the content into several archives.",
            Code::DuplicateName | Code::EditNameExists => "Choose a different name.",
            Code::EditPathNotFound | Code::EditNotAFolder | Code::EditTargetIsFolder => "Refresh the folder and choose an existing target.",
            Code::EditRootProtected | Code::EditResourceUnsupported | Code::EditNoChange => "Choose a different entry or operation.",
            Code::BlobMissing | Code::BlobLengthMismatch => "Upload the file again, then save again.",
            Code::RebuildVerifyFailed => "Nothing was changed. Try saving again; if it keeps failing, report it with the check code.",
            Code::OrphanEntries => "No action needed.",
            Code::DataOverlap => "Check that the file is an unmodified RPF7 archive, or upload a clean copy.",
            _ => "Check that the file is an unmodified RPF7 archive, or upload a clean copy.",
        }
    }
}

impl Code {
    pub fn severity(&self) -> Severity {
        match self {
            Code::OrphanEntries => Severity::Warning,
            _ => Severity::Error,
        }
    }
}
