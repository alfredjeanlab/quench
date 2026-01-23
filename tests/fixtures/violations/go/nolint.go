package violations

import "os"

// VIOLATION: //nolint without justification comment
//nolint:errcheck
func NolintExample() {
	os.Remove("temp.txt")
}
