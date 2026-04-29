package example.service;

// User service.
public class Service {
    // Load one user.
    public User load(String id) {
        return new User(id);
    }

    private String cacheKey(String id) {
        return id;
    }
}

interface UserPort {
    User find(String id);
}
