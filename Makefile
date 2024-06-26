name = $(shell sed -n '/^\[package\]/,/^\[.*\]/s/^name\s*=\s*"\(.*\)"/\1/p' Cargo.toml)

$(name).orc: ../oasis-sdk/tools/orc/orc target/debug/$(name)
	../oasis-sdk/tools/orc/orc init target/debug/$(name) && unzip -p $(name).orc META-INF/MANIFEST.MF | jq .version

../oasis-sdk/tools/orc/orc:
	@cd ../oasis-sdk/tools/orc; \
	go build

target/debug/$(name): Cargo.toml
	cargo build

rebuild:
	@touch Cargo.toml
	@make $(name).orc

localnet:
	@HELA_USE_LOCALNET_CHAINID=1 make rebuild

clean:
	cargo clean
