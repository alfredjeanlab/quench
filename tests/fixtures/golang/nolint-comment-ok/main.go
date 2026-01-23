package main

func main() {
	//nolint:errcheck // reason: error intentionally ignored in startup
	riskyFunction()
}

func riskyFunction() error {
	return nil
}
