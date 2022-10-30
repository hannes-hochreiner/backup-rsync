#test with mockall
test:
  cargo test -F mockall

#format and run clippy
lint:
  cargo fmt
  cargo clippy