# User repository.
class UserRepository:
    # Find one user.
    def find(self, user_id: str) -> dict:
        return {"id": user_id}

    def _cache_key(self, user_id: str) -> str:
        return user_id


# Create a repository.
def create_repository() -> UserRepository:
    return UserRepository()
