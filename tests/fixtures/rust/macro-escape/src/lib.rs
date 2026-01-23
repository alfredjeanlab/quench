// The #[allow] inside the macro definition should be detected
macro_rules! allow_unused {
    ($item:item) => {
        #[allow(dead_code)]
        $item
    };
}

allow_unused!(fn unused() {});
