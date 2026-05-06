# Code Style Guide

Coding conventions for RustChat.

## Rust (Backend)

### Formatting
- Use `cargo fmt` before committing
- Enforced in CI

### Linting
- All code must pass: `cargo clippy --all-targets --all-features -- -D warnings`
- Fix all warnings before PR

### Naming Conventions

| Item | Convention | Example |
|------|------------|---------|
| Types | PascalCase | `UserProfile` |
| Functions | snake_case | `get_user_by_id` |
| Variables | snake_case | `user_count` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_RETRIES` |
| Modules | snake_case | `api/handlers.rs` |

### Error Handling

```rust
// Use thiserror for error types
#[derive(Error, Debug)]
pub enum UserError {
    #[error("User not found: {0}")]
    NotFound(String),
    #[error("Invalid email: {0}")]
    InvalidEmail(String),
}

// Return Result in public functions
pub async fn get_user(id: &str) -> Result<User, UserError> {
    // ...
}
```

### Async Patterns

```rust
// Prefer async fn for IO-bound work
pub async fn fetch_data() -> Result<Data, Error> {
    let response = client.get(url).await?;
    Ok(response.json().await?)
}

// Use tokio::spawn for parallel work
let handles: Vec<_> = items
    .into_iter()
    .map(|item| tokio::spawn(process_item(item)))
    .collect();

for handle in handles {
    handle.await??;
}
```

### Struct Organization

```rust
// Group impl blocks logically
pub struct UserService {
    db: PgPool,
}

// Constructor
impl UserService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

// Public API
impl UserService {
    pub async fn create(&self, input: CreateUser) -> Result<User> {
        // ...
    }
}

// Private helpers
impl UserService {
    async fn validate(&self, input: &CreateUser) -> Result<()> {
        // ...
    }
}
```

## TypeScript/Vue (Frontend)

### Formatting
- Prettier with 2-space indent
- Enforced in CI

### Linting
- ESLint with Vue 3 recommended rules
- No `any` types without justification

### Naming Conventions

| Item | Convention | Example |
|------|------------|---------|
| Components | PascalCase | `UserProfile.vue` |
| Types/Interfaces | PascalCase | `UserProfile` |
| Functions | camelCase | `getUserById` |
| Variables | camelCase | `userCount` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_RETRIES` |
| Composables | camelCase with 'use' | `useUserStore` |

### Vue Component Structure

```vue
<script setup lang="ts">
// 1. Imports
import { ref, computed } from 'vue'
import type { User } from '@/types'

// 2. Props/Emits
const props = defineProps<{
  userId: string
}>()

const emit = defineEmits<{
  updated: [user: User]
}>()

// 3. Composables
const userStore = useUserStore()

// 4. Reactive state
const loading = ref(false)
const error = ref<string | null>(null)

// 5. Computed
const user = computed(() => userStore.getUser(props.userId))

// 6. Methods
async function updateProfile(data: UserUpdate) {
  loading.value = true
  try {
    const updated = await userStore.update(props.userId, data)
    emit('updated', updated)
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <!-- Template here -->
</template>
```

### Type Safety

```typescript
// Avoid 'any'
// Bad
function process(data: any): any { }

// Good
function process(data: UserInput): UserOutput { }

// Use branded types for IDs
type UserId = string & { __brand: 'UserId' }
type ChannelId = string & { __brand: 'ChannelId' }

// Use strict null checks
const user: User | null = await fetchUser(id)
if (user) {
  // TypeScript knows user is not null here
  console.log(user.name)
}
```

## Comments

Comment the *why*, not the *what*:

```rust
// Bad: Restates code
// Increment counter
counter += 1;

// Good: Explains intent
// Rate limit uses sliding window - reset at window boundary,
// not on fixed schedule, to prevent burst attacks at edges
if (now - window_start > WINDOW_SIZE_MS) {
    counter = 0;
}
```

## Testing

### Rust

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_creation() {
        // Arrange
        let service = UserService::new(test_db()).await;
        
        // Act
        let user = service.create(test_input()).await.unwrap();
        
        // Assert
        assert_eq!(user.username, "testuser");
    }
}
```

### TypeScript

```typescript
describe('UserService', () => {
  it('should create a user', async () => {
    // Arrange
    const service = new UserService()
    
    // Act
    const user = await service.create({ username: 'test' })
    
    // Assert
    expect(user.username).toBe('test')
  })
})
```

---

*For more: See [Contributing](./contributing.md)*
