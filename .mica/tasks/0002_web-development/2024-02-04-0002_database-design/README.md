---
title: Database Schema Design
status: Done
assignee: Backend Developer
reviewer: Tech Lead
planned_start: '2026-03-29'
planned_end: '2026-04-02'
actual_start: '2024-02-04'
actual_end: '2024-02-07'
dependencies:
  - 0002_web-development/2024-02-01-0001_project-setup
pinned: false
---
## Database Schema Design

Design and implement the database schema for the web application.

### Database Schema

#### Users Table
```sql
CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  email VARCHAR(255) UNIQUE NOT NULL,
  password_hash VARCHAR(255) NOT NULL,
  first_name VARCHAR(100) NOT NULL,
  last_name VARCHAR(100) NOT NULL,
  role VARCHAR(50) DEFAULT 'user',
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

#### Products Table
```sql
CREATE TABLE products (
  id SERIAL PRIMARY KEY,
  name VARCHAR(255) NOT NULL,
  description TEXT,
  price DECIMAL(10,2) NOT NULL,
  category_id INTEGER REFERENCES categories(id),
  stock_quantity INTEGER DEFAULT 0,
  image_url VARCHAR(500),
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

#### Orders Table
```sql
CREATE TABLE orders (
  id SERIAL PRIMARY KEY,
  user_id INTEGER REFERENCES users(id),
  total_amount DECIMAL(10,2) NOT NULL,
  status VARCHAR(50) DEFAULT 'pending',
  shipping_address TEXT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### Entity Relationship Diagram

The database follows a typical e-commerce pattern:

- **Users** can place multiple **Orders**
- **Orders** contain multiple **Order Items**
- **Products** belong to **Categories**
- **Products** can have multiple **Reviews**

### Performance Considerations

1. **Indexing Strategy**
   - Primary keys (automatic)
   - Foreign keys for joins
   - Email for user lookups
   - Product name for search

2. **Data Normalization**
   - 3NF compliance
   - Separate categories table
   - Audit fields for tracking changes

### Migration Scripts

Migration files created for:
- ✅ Initial schema creation
- ✅ Seed data insertion
- ✅ Index creation
- ✅ Constraints and triggers

### Testing

Database tests implemented for:
- Schema validation
- Constraint enforcement
- Data integrity
- Performance benchmarks
