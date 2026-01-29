# Release new version (tag + push)

release-check:
    cargo test --all --all-features
    cargo build --release --all-features
    cargo publish --dry-run

release: release-check
    version=$(node -p "require('./package.json').version"); \
    git tag "v${version}"; \
    git push origin "v${version}"
