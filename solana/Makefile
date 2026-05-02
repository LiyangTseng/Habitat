SHELL := /bin/bash

PROGRAM_NAME := habitat_settlement_program
IDL_PATH := target/idl/$(PROGRAM_NAME).json

.PHONY: build test deploy-local idl localnet-bootstrap

build:
	anchor build

test:
	anchor test --skip-local-validator

deploy-local:
	anchor deploy --provider.cluster localnet

idl:
	anchor build
	@test -f "$(IDL_PATH)" || (echo "IDL not found at $(IDL_PATH)" && exit 1)
	@echo "IDL generated at $(IDL_PATH)"

localnet-bootstrap:
	./scripts/bootstrap-local-validator.sh
