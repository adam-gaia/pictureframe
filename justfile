build:
    trunk build --dist ./dist/viewer crates/frontend-viewer/index.html --public-url /
    trunk build --dist ./dist/admin crates/frontend-admin/index.html --public-url /admin/
    cargo lbuild

run: build
    RUST_LOG=debug cargo lrun -- --data-dir ./data --dist-dir ./dist
