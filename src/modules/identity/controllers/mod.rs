//! HTTP handlers (Loco controllers). Private to the module: expose routes through `super::routes`.

use loco_rs::controller::Routes;

pub(super) fn routes() -> Vec<Routes> {
    Vec::new()
}
