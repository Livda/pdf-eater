/// Erreurs métier de la couche PDF.
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("Page {page} invalide (le document a {total} page(s))")]
    PageOutOfRange { page: u32, total: u32 },

    #[error("Impossible de supprimer toutes les pages du document")]
    WouldDeleteAll,

    #[error("Page {0} apparaît plusieurs fois")]
    DuplicatePage(u32),

    #[error("L'ordre doit contenir exactement {expected} page(s), {got} fournie(s)")]
    WrongPageCount { expected: usize, got: usize },

    #[error("Aucune page spécifiée")]
    NoPages,

    #[error("Plage de pages invalide : '{0}'")]
    InvalidRange(String),

    #[error("Numéro de page invalide : '{0}'")]
    InvalidPageNumber(String),

    #[error(transparent)]
    Lopdf(#[from] lopdf::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Alias de résultat local pour éviter la répétition du type d'erreur.
pub type Result<T> = std::result::Result<T, PdfError>;
