package service

// User is a domain user.
type User struct {
	ID string
}

// Store loads users.
type Store interface {
	Load(id string) User
}

// NewStore creates a store.
func NewStore() Store {
	return nil
}

func helper() {}

// Load attaches to User.
func (u User) Load(id string) User {
	return u
}
