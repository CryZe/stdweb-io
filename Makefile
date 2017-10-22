asmjs:
	@cargo rustc --release -p stdweb-io-html --target asmjs-unknown-emscripten -- -C opt-level=z
	@cp target/asmjs-unknown-emscripten/release/stdweb-io-html*.js* stdweb-io.js

debug:
	@cargo build  -p stdweb-io-html --target asmjs-unknown-emscripten
	@cp target/asmjs-unknown-emscripten/debug/stdweb-io-html*.js* stdweb-io.js

run:
	@python -m SimpleHTTPServer 8080
