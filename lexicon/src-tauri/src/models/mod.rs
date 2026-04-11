pub mod addon;
pub mod annotation;
pub mod book;
pub mod discover;
pub mod download;
pub mod reader;
pub mod search;

pub use addon::{AddonDescriptor, AddonRole, AddonSettingEntry};
pub use annotation::{Annotation, NewAnnotation};
pub use book::{Book, BOOK_STATUS_FINISHED, BOOK_STATUS_READING, BOOK_STATUS_UNREAD};
pub use discover::{
    DiscoverCatalog, DiscoverCatalogItem, DiscoverCatalogPageResponse, DiscoverItemDetails,
    PluginErrorKind, PluginTypedError, SourceDownloadResult, SourcePluginInfo,
    SourceSearchResultGroup,
};
pub use download::{
    DownloadProgressEvent, DownloadRecord, DownloadStateEvent, StartDownloadRequest,
};
pub use reader::{BookContent, PdfDocumentData};
pub use search::SearchBookResult;
