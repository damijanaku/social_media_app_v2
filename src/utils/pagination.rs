use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    #[serde(default)]
    pub with_count: bool,
}

impl PaginationParams {
    pub fn resolve(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let limit = self.limit.unwrap_or(10).max(1).min(100);
        (page, limit)
    }

    pub fn offset(&self) -> i64 {
        let (page, limit) = self.resolve();
        (page - 1) * limit
    }

    pub fn limit(&self) -> i64 {
        self.resolve().1
    }

    pub fn page(&self) -> i64 {
        self.resolve().0
    }

    pub fn fetch_limit(&self) -> i64 {
        self.limit() + 1
    }

    pub fn include_count(&self) -> bool {
        self.with_count
    }
}

#[derive(Debug, serde::Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub meta: PaginationMeta,
}

#[derive(Debug, serde::Serialize)]
pub struct PaginationMeta {
    pub page: i64,
    pub limit: i64,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_items: Option<i64>,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, page: i64, limit: i64, has_more: bool) -> Self {
        Self {
            items,
            meta: PaginationMeta {
                page,
                limit,
                has_more,
                total_items: None,
            },
        }
    }

    pub fn with_total(mut self, total: i64) -> Self {
        self.meta.total_items = Some(total);
        self
    }
}
