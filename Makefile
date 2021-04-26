dev-book:
	mdbook watch docs --open

prettier:
	npx prettier **/*.md --write
