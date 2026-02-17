# Sprint 6: Type System and Reference Forms

## Sprint Overview

**Sprint Goal**: Add complete type grammar and catalog/object reference syntax.

**Sprint Duration**: TBD

**Status**: 🔵 **Planned**

**Dependencies**:
- Sprint 1 (Diagnostics and Span Infrastructure) ✅
- Sprint 2 (Lexer Core and Token Model) ✅
- Sprint 3 (Parser Skeleton and Recovery Framework) ✅
- Sprint 4 (Program, Session, Transaction, Catalog Statements) ✅
- Sprint 5 (Values, Literals, and Expression Core) - Required

## Scope

This sprint implements the complete type system that forms the semantic backbone of GQL. The type system provides type annotations, type constructors, and type constraints used throughout the language in variable declarations, CAST expressions, IS TYPED predicates, graph schema definitions, and more. Additionally, this sprint completes the implementation of all catalog/object reference forms including schema references, graph references, graph type references, binding table references, and procedure references.

### Feature Coverage from GQL_FEATURES.md

Sprint 6 maps to the following sections in `GQL_FEATURES.md`:

1. **Section 14: Type System** (Lines 852-1063)
   - Value types hierarchy
   - Predefined types:
     - Boolean types (BOOL, BOOLEAN)
     - String types (STRING, CHAR, VARCHAR, BYTES, BINARY, VARBINARY)
     - Numeric types:
       - Exact numeric (signed/unsigned binary: INT8, INT16, INT32, INT64, INT128, INT256, SMALLINT, INT, BIGINT, UINT8, etc.)
       - Exact numeric (decimal: DECIMAL, DEC with precision/scale)
       - Approximate numeric (FLOAT16, FLOAT32, FLOAT64, FLOAT128, FLOAT256, REAL, DOUBLE PRECISION)
     - Temporal types:
       - Instant types (ZONED DATETIME, LOCAL DATETIME, DATE, ZONED TIME, LOCAL TIME, TIMESTAMP)
       - Duration types (DURATION, DURATION YEAR TO MONTH, DURATION DAY TO SECOND)
     - Reference value types:
       - Graph reference types (ANY PROPERTY GRAPH, PROPERTY GRAPH with nested spec)
       - Binding table reference types (BINDING TABLE with field types)
       - Node reference types (ANY NODE, VERTEX, with type specifications)
       - Edge reference types (ANY EDGE, RELATIONSHIP, with type specifications)
     - Immaterial value types (NULL, NOTHING)
   - Constructed types:
     - Path value types (PATH)
     - List value types (LIST<T>, ARRAY<T>, T LIST, T ARRAY)
     - Record types (ANY RECORD, RECORD with field types)
   - Type modifiers:
     - Type annotation operator (::, TYPED keyword)
     - NOT NULL constraint
   - Dynamic union types (open/closed)

2. **Section 19: Variables, Parameters & References** (Lines 1567-1642) - Catalog Reference Forms
   - Schema references:
     - Absolute schema paths (/)
     - Relative schema paths (../)
     - Predefined schema references (HOME_SCHEMA, CURRENT_SCHEMA, .)
     - Reference parameter form ($$name)
   - Graph references:
     - Catalog-qualified graph names
     - Delimited graph names
     - Home graph references (HOME_GRAPH, HOME_PROPERTY_GRAPH)
     - Reference parameter form
   - Graph type references:
     - Catalog-qualified graph type names
     - Reference parameter form
   - Binding table references:
     - Catalog-qualified binding table names
     - Delimited binding table names
     - Reference parameter form
   - Procedure references:
     - Catalog-qualified procedure names
     - Reference parameter form
   - Catalog parent paths (directory/schema/object path components)

## Exit Criteria

- [ ] All type categories parse with correct AST forms
- [ ] Predefined types (boolean, string, numeric, temporal, reference, immaterial) are parsed
- [ ] Constructed types (path, list, record) parse with proper structure
- [ ] Type modifiers (NOT NULL, :: type annotation) work correctly
- [ ] List types support all syntax variants (LIST<T>, ARRAY<T>, T LIST, T ARRAY)
- [ ] Record types parse field type specifications
- [ ] Reference value types (graph, binding table, node, edge) parse with all variants
- [ ] All catalog/object reference forms parse correctly
- [ ] Schema references handle absolute paths, relative paths, and predefined forms
- [ ] Type parser integrates with CAST expressions from Sprint 5
- [ ] Type parser integrates with IS TYPED predicates from Sprint 5
- [ ] Type parser provides foundation for variable declarations (Sprint 7+)
- [ ] Type parser provides foundation for graph type specifications (Sprint 12)
- [ ] Parser produces structured diagnostics for malformed types
- [ ] AST nodes have proper span information for all type components
- [ ] Recovery mechanisms handle errors at type boundaries
- [ ] Unit tests cover all type variants and error cases
- [ ] Type parsing is reusable across all contexts (CAST, IS TYPED, variable declarations, etc.)

## Implementation Tasks

### Task 1: AST Node Definitions for Predefined Types

**Description**: Define AST types for all predefined type forms.

**Deliverables**:
- `ValueType` enum representing the top-level type union
- `PredefinedType` enum with variants:
  - `Boolean(BooleanType)` - BOOL, BOOLEAN
  - `CharacterString(CharacterStringType)` - STRING, CHAR(n), VARCHAR(n)
  - `ByteString(ByteStringType)` - BYTES, BINARY(n), VARBINARY(n)
  - `Numeric(NumericType)` - all numeric types
  - `Temporal(TemporalType)` - all temporal types
  - `ReferenceValue(ReferenceValueType)` - graph, node, edge, binding table references
  - `Immaterial(ImmaterialValueType)` - NULL, NOTHING
- `BooleanType` enum: Bool, Boolean
- `CharacterStringType` enum:
  - `String` - STRING
  - `Char(Option<u32>)` - CHAR(n), CHAR
  - `VarChar(Option<u32>)` - VARCHAR(n), VARCHAR
- `ByteStringType` enum:
  - `Bytes` - BYTES
  - `Binary(Option<u32>)` - BINARY(n), BINARY
  - `VarBinary(Option<u32>)` - VARBINARY(n), VARBINARY
- `NumericType` enum:
  - `Exact(ExactNumericType)` - exact numeric types
  - `Approximate(ApproximateNumericType)` - approximate numeric types
- `ExactNumericType` enum:
  - `SignedBinary(SignedBinaryExactNumericType)` - INT8, INT16, INT32, INT64, INT128, INT256, SMALLINT, INT, INTEGER, BIGINT, SIGNED variants
  - `UnsignedBinary(UnsignedBinaryExactNumericType)` - UINT8, UINT16, UINT32, UINT64, UINT128, UINT256, USMALLINT, UINT, UBIGINT, UNSIGNED variants
  - `Decimal(DecimalExactNumericType)` - DECIMAL(p, s), DEC(p, s)
- `SignedBinaryExactNumericType` enum: Int8, Int16, Int32, Int64, Int128, Int256, SmallInt, Int, Integer, BigInt, SignedInt8, SignedInt16, etc.
- `UnsignedBinaryExactNumericType` enum: UInt8, UInt16, UInt32, UInt64, UInt128, UInt256, USmallInt, UInt, UBigInt, UnsignedInt8, UnsignedInt16, etc.
- `DecimalExactNumericType` struct:
  - `kind: DecimalKind` - DECIMAL or DEC
  - `precision: Option<u32>`
  - `scale: Option<u32>`
  - `span: Span`
- `ApproximateNumericType` enum: Float16, Float32, Float64, Float128, Float256, Float(Option<u32>), Real, DoublePrecision
- `TemporalType` enum:
  - `Instant(TemporalInstantType)` - datetime, date, time types
  - `Duration(TemporalDurationType)` - duration types
- `TemporalInstantType` enum:
  - `ZonedDatetime` - ZONED DATETIME, TIMESTAMP WITH TIME ZONE
  - `LocalDatetime` - LOCAL DATETIME, TIMESTAMP, TIMESTAMP WITHOUT TIME ZONE
  - `Date` - DATE
  - `ZonedTime` - ZONED TIME, TIME WITH TIME ZONE
  - `LocalTime` - LOCAL TIME, TIME, TIME WITHOUT TIME ZONE
- `TemporalDurationType` enum:
  - `Duration` - DURATION
  - `DurationYearToMonth` - DURATION YEAR TO MONTH
  - `DurationDayToSecond` - DURATION DAY TO SECOND
- `ImmaterialValueType` enum:
  - `Null` - NULL
  - `NullNotNull` - NULL NOT NULL
  - `Nothing` - NOTHING

**Grammar References**:
- `valueType` (Line 1719)
- `predefinedType` (Line 1740)
- `booleanType` (Line 1750)
- `characterStringType` (Line 1754)
- `byteStringType` (Line 1760)
- `numericType` (Line 1778)
- `exactNumericType` (Line 1783)
- `signedBinaryExactNumericType` (Line 1793)
- `unsignedBinaryExactNumericType` (Line 1806)
- `decimalExactNumericType` (Line 1831)
- `approximateNumericType` (Line 1843)
- `temporalType` (Line 1854)
- `temporalInstantType` (Line 1859)
- `datetimeType` (Line 1867)
- `localdatetimeType` (Line 1872)
- `dateType` (Line 1877)
- `timeType` (Line 1881)
- `localtimeType` (Line 1886)
- `temporalDurationType` (Line 1891)
- `durationType` (Line 1891)
- `immaterialValueType` (Line 1907)
- `nullType` (Line 1912)
- `emptyType` (Line 1916)

**Acceptance Criteria**:
- [ ] All predefined type AST nodes defined in `src/ast/types.rs` (new module)
- [ ] Each type node has `Span` information
- [ ] Nodes implement necessary traits (Debug, Clone, PartialEq)
- [ ] Documentation comments explain each type variant
- [ ] Numeric types with precision/scale parameters preserve values
- [ ] Temporal types distinguish zoned vs local variants

**File Location**: `src/ast/types.rs` (new file)

---

### Task 2: AST Node Definitions for Reference Value Types

**Description**: Define AST types for reference value types (graph, node, edge, binding table).

**Deliverables**:
- `ReferenceValueType` enum with variants:
  - `Graph(GraphReferenceValueType)` - graph reference types
  - `BindingTable(BindingTableReferenceValueType)` - binding table reference types
  - `Node(NodeReferenceValueType)` - node reference types
  - `Edge(EdgeReferenceValueType)` - edge reference types
- `GraphReferenceValueType` enum:
  - `AnyPropertyGraph { not_null: bool, span: Span }` - ANY [PROPERTY] GRAPH [NOT NULL]
  - `PropertyGraph { spec: Box<NestedGraphTypeSpecification>, not_null: bool, span: Span }` - PROPERTY GRAPH <nested_spec> [NOT NULL]
- `BindingTableReferenceValueType` struct:
  - `field_types: FieldTypesSpecification` - field type specifications
  - `not_null: bool`
  - `span: Span`
- `NodeReferenceValueType` enum:
  - `Any { use_vertex: bool, not_null: bool, span: Span }` - [ANY] NODE [NOT NULL] or [ANY] VERTEX [NOT NULL]
  - `Typed { spec: Box<NodeTypeSpecification>, not_null: bool, span: Span }` - <node_type_spec> [NOT NULL]
- `EdgeReferenceValueType` enum:
  - `Any { use_relationship: bool, not_null: bool, span: Span }` - [ANY] EDGE [NOT NULL] or [ANY] RELATIONSHIP [NOT NULL]
  - `Typed { spec: Box<EdgeTypeSpecification>, not_null: bool, span: Span }` - <edge_type_spec> [NOT NULL]
- `FieldTypesSpecification` struct - field type list (placeholder for Sprint 12)
- `NestedGraphTypeSpecification` struct - nested graph type (placeholder for Sprint 12)
- `NodeTypeSpecification` struct - node type specification (placeholder for Sprint 12)
- `EdgeTypeSpecification` struct - edge type specification (placeholder for Sprint 12)

**Grammar References**:
- `referenceValueType` (Line 1900)
- `graphReferenceValueType` (Line 1921)
- `bindingTableReferenceValueType` (Line 1934)
- `nodeReferenceValueType` (Line 1938)
- `edgeReferenceValueType` (Line 1951)
- `fieldTypesSpecification` (Line 1982)

**Acceptance Criteria**:
- [ ] All reference value type AST nodes defined
- [ ] NOT NULL modifier tracked for all reference types
- [ ] PROPERTY keyword optional for graph types
- [ ] NODE/VERTEX and EDGE/RELATIONSHIP synonyms supported
- [ ] Placeholder types for nested specifications documented
- [ ] Span information captures entire type extent
- [ ] Integration points for Sprint 12 (graph type specifications) clear

**File Location**: `src/ast/types.rs`

---

### Task 3: AST Node Definitions for Constructed Types

**Description**: Define AST types for path, list, and record types.

**Deliverables**:
- `PathValueType` struct:
  - `span: Span`
- `ListValueType` struct:
  - `element_type: Box<ValueType>`
  - `syntax_form: ListSyntaxForm` - LIST<T>, ARRAY<T>, T LIST, T ARRAY
  - `span: Span`
- `ListSyntaxForm` enum:
  - `List` - LIST<T>
  - `Array` - ARRAY<T>
  - `PostfixList` - T LIST
  - `PostfixArray` - T ARRAY
- `RecordType` enum:
  - `AnyRecord { span: Span }` - ANY RECORD
  - `Record { field_types: FieldTypesSpecification, span: Span }` - RECORD with fields
- `FieldTypesSpecification` struct:
  - `fields: Vec<FieldType>`
  - `span: Span`
- `FieldType` struct:
  - `field_name: SmolStr`
  - `field_type: Box<ValueType>`
  - `span: Span`

**Grammar References**:
- `pathValueType` (Line 1964)
- `listValueTypeName` (Line 1968)
- `recordType` (Line 1977)
- `fieldTypesSpecification` (Line 1982)
- `fieldType` (Line 1996)

**Acceptance Criteria**:
- [ ] Path type AST node defined
- [ ] List type supports all four syntax forms (prefix and postfix)
- [ ] Record type distinguishes ANY RECORD vs typed RECORD
- [ ] Field types have name and type specification
- [ ] Field type separator (::) captured in parsing
- [ ] Recursive type structure supports nested types
- [ ] Span tracking covers entire type construct

**File Location**: `src/ast/types.rs`

---

### Task 4: AST Node Definitions for Type Modifiers

**Description**: Define AST types for type modifiers and annotations.

**Deliverables**:
- `TypeAnnotation` struct:
  - `operator: TypeAnnotationOperator` - :: or TYPED
  - `type_ref: Box<ValueType>`
  - `span: Span`
- `TypeAnnotationOperator` enum:
  - `DoubleColon` - ::
  - `Typed` - TYPED keyword
- `NotNullConstraint` struct:
  - `span: Span`
- Update `ValueType` to optionally carry NOT NULL modifier:
  - Consider wrapping types: `TypeWithModifiers` struct with `base_type: ValueType`, `not_null: bool`

**Grammar References**:
- `typed` (Line 1735)
- `notNull` (Line 1990)

**Acceptance Criteria**:
- [ ] Type annotation supports :: and TYPED keyword forms
- [ ] NOT NULL constraint can be applied to types
- [ ] Type modifiers compose with base types
- [ ] Span information captures modifiers
- [ ] Parser can distinguish `expr :: type` (annotation) from `field :: type` (record field)

**File Location**: `src/ast/types.rs`

---

### Task 5: AST Node Definitions for Catalog/Object References

**Description**: Define AST types for all catalog and object reference forms.

**Deliverables**:
- `SchemaReference` enum:
  - `AbsolutePath(Vec<SmolStr>)` - / path components
  - `RelativePath(Vec<SmolStr>)` - ../ path components
  - `HomeSchema` - HOME_SCHEMA
  - `CurrentSchema` - CURRENT_SCHEMA
  - `Dot` - .
  - `ReferenceParameter(SmolStr)` - $$name
  - Each variant includes `span: Span`
- `GraphReference` enum:
  - `CatalogQualified(CatalogQualifiedName)` - schema::graph
  - `Delimited(SmolStr)` - delimited identifier
  - `HomeGraph` - HOME_GRAPH
  - `HomePropertyGraph` - HOME_PROPERTY_GRAPH
  - `ReferenceParameter(SmolStr)` - $$name
  - Each variant includes `span: Span`
- `GraphTypeReference` enum:
  - `CatalogQualified(CatalogQualifiedName)` - schema::type_name
  - `ReferenceParameter(SmolStr)` - $$name
  - Each variant includes `span: Span`
- `BindingTableReference` enum:
  - `CatalogQualified(CatalogQualifiedName)` - schema::table_name
  - `Delimited(SmolStr)` - delimited identifier
  - `ReferenceParameter(SmolStr)` - $$name
  - Each variant includes `span: Span`
- `ProcedureReference` enum:
  - `CatalogQualified(CatalogQualifiedName)` - schema::procedure_name
  - `ReferenceParameter(SmolStr)` - $$name
  - Each variant includes `span: Span`
- `CatalogQualifiedName` struct:
  - `parent: Option<CatalogObjectParentReference>`
  - `name: SmolStr`
  - `span: Span`
- `CatalogObjectParentReference` enum:
  - `Schema(SchemaReference)` - schema reference as parent
  - `Object(Box<CatalogQualifiedName>)` - another qualified name as parent
  - Each variant includes `span: Span`

**Grammar References**:
- `schemaReference` (Line 1381)
- `graphReference` (Line 1421)
- `graphTypeReference` (Line 1439)
- `bindingTableReference` (Line 1450)
- `procedureReference` (Line 1458)
- `catalogObjectParentReference` (Line 1469)

**Acceptance Criteria**:
- [ ] All reference forms have AST representations
- [ ] Schema references support absolute paths (/)
- [ ] Schema references support relative paths (../)
- [ ] Predefined references (HOME_SCHEMA, CURRENT_SCHEMA, etc.) recognized
- [ ] Catalog-qualified names parse with optional parent references
- [ ] Reference parameters ($$name) distinguished from value parameters ($name)
- [ ] Span tracking covers entire reference path
- [ ] Documentation explains each reference variant

**File Location**: `src/ast/references.rs` (new file)

---

### Task 6: Lexer Extensions for Type System Tokens

**Description**: Ensure lexer supports all tokens needed for type system.

**Deliverables**:
- Verify existing type keywords are sufficient:
  - Boolean: BOOL, BOOLEAN
  - String: STRING, CHAR, VARCHAR, BYTES, BINARY, VARBINARY
  - Numeric: INT8, INT16, INT32, INT64, INT128, INT256, UINT8, UINT16, UINT32, UINT64, UINT128, UINT256, SMALLINT, INT, INTEGER, BIGINT, USMALLINT, UINT, UBIGINT, DECIMAL, DEC, FLOAT16, FLOAT32, FLOAT64, FLOAT128, FLOAT256, FLOAT, REAL, DOUBLE, PRECISION, SIGNED, UNSIGNED
  - Temporal: ZONED, LOCAL, DATETIME, TIMESTAMP, DATE, TIME, DURATION, YEAR, MONTH, DAY, SECOND, WITH, WITHOUT, ZONE
  - Reference: GRAPH, NODE, VERTEX, EDGE, RELATIONSHIP, TABLE, BINDING, ANY, PROPERTY
  - Immaterial: NULL, NOTHING
  - Constructed: PATH, LIST, ARRAY, RECORD
  - Modifiers: TYPED, NOT
- Add any missing type keywords to keyword table
- Ensure :: operator is tokenized (double colon for type annotation)
- Verify numeric literal parsing for precision/scale (used in DECIMAL(p, s))
- Ensure schema path operators (/, .., .) are tokenized correctly

**Lexer Enhancements Needed**:
- Add missing numeric type keywords (INT128, INT256, UINT128, UINT256, FLOAT16, FLOAT128, FLOAT256, etc.)
- Add temporal keywords (ZONED, LOCAL, if missing)
- Add reference type keywords (VERTEX, RELATIONSHIP, if missing)
- Add NOTHING keyword
- Ensure :: operator distinct from : (single colon)
- Ensure / operator for schema paths doesn't conflict with division
- Ensure .. operator for relative paths tokenized correctly

**Grammar References**:
- Type keyword definitions throughout Lines 1750-1992
- Schema reference operators (Line 1381)
- Type annotation operator (Line 1735)

**Acceptance Criteria**:
- [ ] All type keywords tokenized correctly
- [ ] Case-insensitive keyword matching works
- [ ] :: operator tokenized as single token (not two colons)
- [ ] Schema path operators (/, .., .) tokenized correctly
- [ ] Numeric literals with precision/scale parse correctly
- [ ] No new lexer errors introduced
- [ ] All type-related tokens have proper span information

**File Location**: `src/lexer/keywords.rs`, `src/lexer/mod.rs`, `src/lexer/token.rs`

---

### Task 7: Type Parser - Predefined Types

**Description**: Implement parsing for all predefined type forms.

**Deliverables**:
- `parse_value_type()` - entry point for type parsing
- `parse_predefined_type()` - dispatch to specific predefined type parsers
- Parser functions for each predefined type category:
  - `parse_boolean_type()` - BOOL, BOOLEAN
  - `parse_character_string_type()` - STRING, CHAR(n), VARCHAR(n)
  - `parse_byte_string_type()` - BYTES, BINARY(n), VARBINARY(n)
  - `parse_numeric_type()` - all numeric types
  - `parse_exact_numeric_type()` - signed/unsigned/decimal numeric types
  - `parse_signed_binary_exact_numeric_type()` - INT8, SMALLINT, etc.
  - `parse_unsigned_binary_exact_numeric_type()` - UINT8, USMALLINT, etc.
  - `parse_decimal_exact_numeric_type()` - DECIMAL(p, s), DEC(p, s)
  - `parse_approximate_numeric_type()` - FLOAT16, REAL, DOUBLE PRECISION, etc.
  - `parse_temporal_type()` - all temporal types
  - `parse_temporal_instant_type()` - datetime, date, time types
  - `parse_temporal_duration_type()` - duration types
  - `parse_immaterial_value_type()` - NULL, NOTHING
- Parameter parsing for parameterized types (CHAR(n), DECIMAL(p, s), FLOAT(p))
- Synonym handling (INTEGER = INT, DEC = DECIMAL, etc.)
- Error diagnostics for malformed type specifications

**Grammar References**:
- `valueType` (Line 1719)
- `predefinedType` (Line 1740)
- Various type-specific rules (Lines 1750-1898)

**Acceptance Criteria**:
- [ ] All predefined type forms parse to correct AST nodes
- [ ] Parameterized types parse with correct parameter values
- [ ] Type synonyms (BOOL/BOOLEAN, INT/INTEGER, DEC/DECIMAL) handled
- [ ] DOUBLE PRECISION parsed as multi-word keyword
- [ ] TIMESTAMP WITH/WITHOUT TIME ZONE variants parsed correctly
- [ ] DURATION YEAR TO MONTH and DURATION DAY TO SECOND parsed correctly
- [ ] Error recovery on malformed type specifications
- [ ] Comprehensive unit tests for each type category

**File Location**: `src/parser/types.rs` (new module)

---

### Task 8: Type Parser - Reference Value Types

**Description**: Implement parsing for reference value types (graph, node, edge, binding table).

**Deliverables**:
- `parse_reference_value_type()` - dispatch to specific reference type parsers
- Parser functions for each reference value type:
  - `parse_graph_reference_value_type()` - ANY PROPERTY GRAPH, PROPERTY GRAPH <spec>
  - `parse_binding_table_reference_value_type()` - BINDING TABLE <field_types>
  - `parse_node_reference_value_type()` - ANY NODE, ANY VERTEX, <node_type_spec>
  - `parse_edge_reference_value_type()` - ANY EDGE, ANY RELATIONSHIP, <edge_type_spec>
- `parse_field_types_specification()` - parse field type lists
- `parse_field_type()` - parse individual field :: type
- NOT NULL modifier parsing for all reference types
- PROPERTY keyword handling (optional)
- NODE/VERTEX and EDGE/RELATIONSHIP synonym handling
- Placeholder parsing for nested type specifications (Sprint 12 will implement fully)

**Grammar References**:
- `referenceValueType` (Line 1900)
- `graphReferenceValueType` (Line 1921)
- `bindingTableReferenceValueType` (Line 1934)
- `nodeReferenceValueType` (Line 1938)
- `edgeReferenceValueType` (Line 1951)
- `fieldTypesSpecification` (Line 1982)
- `fieldType` (Line 1996)

**Acceptance Criteria**:
- [ ] All reference value type forms parse correctly
- [ ] NOT NULL modifier properly attached to reference types
- [ ] PROPERTY keyword optional for graph types
- [ ] ANY keyword optional for node/edge types
- [ ] Synonyms (NODE/VERTEX, EDGE/RELATIONSHIP) handled
- [ ] Field type specifications parse with :: separator
- [ ] Nested type specifications use placeholder (defer to Sprint 12)
- [ ] Error diagnostics for malformed reference types
- [ ] Unit tests for each reference type variant

**File Location**: `src/parser/types.rs`

---

### Task 9: Type Parser - Constructed Types

**Description**: Implement parsing for path, list, and record types.

**Deliverables**:
- Parser functions for constructed types:
  - `parse_path_value_type()` - PATH
  - `parse_list_value_type()` - LIST<T>, ARRAY<T>, T LIST, T ARRAY
  - `parse_record_type()` - ANY RECORD, RECORD { field_types }
- List syntax form disambiguation:
  - Prefix forms: LIST<T>, ARRAY<T>
  - Postfix forms: T LIST, T ARRAY
- Recursive type parsing (list element types, record field types)
- Field type parsing for records
- Empty record type handling

**Grammar References**:
- `pathValueType` (Line 1964)
- `listValueTypeName` (Line 1968)
- `recordType` (Line 1977)

**Acceptance Criteria**:
- [ ] Path type parses correctly
- [ ] All four list syntax forms parse to correct AST
- [ ] List types recursively parse element types
- [ ] Record types parse field type specifications
- [ ] ANY RECORD distinguished from typed RECORD
- [ ] Nested types (e.g., LIST<LIST<INT>>) parse correctly
- [ ] Error recovery on malformed constructed types
- [ ] Unit tests for each constructed type variant

**File Location**: `src/parser/types.rs`

---

### Task 10: Type Parser - Type Modifiers and Annotations

**Description**: Implement parsing for type modifiers and annotations.

**Deliverables**:
- `parse_type_annotation()` - parse :: type or TYPED type
- `parse_not_null_constraint()` - parse NOT NULL modifier
- Integration with expression parser (distinguish `expr :: type` from `field :: type`)
- Type modifier composition (e.g., `INT NOT NULL`)
- Precedence handling (NOT NULL applies after base type)

**Grammar References**:
- `typed` (Line 1735)
- `notNull` (Line 1990)

**Acceptance Criteria**:
- [ ] Type annotation :: operator parses correctly
- [ ] TYPED keyword form works as alternative to ::
- [ ] NOT NULL modifier attaches to types
- [ ] Parser distinguishes type annotation context from expression context
- [ ] Type modifiers compose correctly with base types
- [ ] Error diagnostics for missing type after ::
- [ ] Unit tests for type modifiers

**File Location**: `src/parser/types.rs`

---

### Task 11: Catalog/Object Reference Parser

**Description**: Implement parsing for all catalog and object reference forms.

**Deliverables**:
- Parser functions for each reference type:
  - `parse_schema_reference()` - all schema reference variants
  - `parse_graph_reference()` - all graph reference variants
  - `parse_graph_type_reference()` - all graph type reference variants
  - `parse_binding_table_reference()` - all binding table reference variants
  - `parse_procedure_reference()` - all procedure reference variants
- `parse_catalog_qualified_name()` - parse schema::name forms
- `parse_catalog_object_parent_reference()` - parse parent qualification
- Schema path parsing (absolute /, relative ../)
- Predefined reference parsing (HOME_SCHEMA, CURRENT_SCHEMA, HOME_GRAPH, etc.)
- Reference parameter parsing ($$name)
- Delimited identifier support
- Multi-level catalog qualification (e.g., schema::parent::name)

**Grammar References**:
- `schemaReference` (Line 1381)
- `graphReference` (Line 1421)
- `graphTypeReference` (Line 1439)
- `bindingTableReference` (Line 1450)
- `procedureReference` (Line 1458)
- `catalogObjectParentReference` (Line 1469)

**Acceptance Criteria**:
- [ ] All schema reference forms parse correctly
- [ ] Absolute schema paths (/) parse with path components
- [ ] Relative schema paths (../) parse correctly
- [ ] Predefined references (HOME_SCHEMA, CURRENT_SCHEMA, ., etc.) recognized
- [ ] Catalog-qualified names parse with :: separator
- [ ] Multi-level qualification works (parent::child::name)
- [ ] Reference parameters ($$name) parse and distinguish from value parameters ($name)
- [ ] Delimited identifiers work in all reference contexts
- [ ] Error diagnostics for malformed references
- [ ] Unit tests for all reference variants

**File Location**: `src/parser/references.rs` (new module)

---

### Task 12: Integration with Expression Parser (Sprint 5)

**Description**: Integrate type parser with expression parser from Sprint 5.

**Deliverables**:
- Update CAST expressions to use real type parser (replace placeholder)
- Update IS TYPED predicates to use real type parser (replace placeholder)
- Update type annotation in expressions (expr :: type)
- Ensure type parsing doesn't conflict with expression parsing
- Test CAST(expr AS type) with all type variants
- Test IS TYPED type with all type variants
- Test expr :: type annotations

**Grammar References**:
- `castSpecification` (Line 2365)
- `valueTypePredicate` (Line 2052)
- `typed` (Line 1735)

**Acceptance Criteria**:
- [ ] CAST expressions parse with real type specifications
- [ ] IS TYPED predicates parse with real type specifications
- [ ] Expression :: type annotations work correctly
- [ ] No parser conflicts between expression and type parsing
- [ ] All CAST/IS TYPED tests from Sprint 5 updated to use real types
- [ ] Integration tests validate end-to-end parsing
- [ ] Type parsing is context-aware (knows when parsing type vs expression)

**File Location**: `src/parser/expression.rs`, `src/parser/types.rs`

---

### Task 13: Integration with Statement Parsing (Sprint 4)

**Description**: Integrate type and reference parsers with statement parsers from Sprint 4.

**Deliverables**:
- Update session/transaction/catalog parsers to use real reference parsers:
  - Session SET SCHEMA uses SchemaReference parser
  - Session SET GRAPH uses GraphReference parser
  - CREATE/DROP SCHEMA uses SchemaReference parser
  - CREATE/DROP GRAPH uses GraphReference parser
  - CREATE/DROP GRAPH TYPE uses GraphTypeReference parser
- Replace placeholder reference types in existing AST
- Ensure all statement tests work with real references
- Test catalog operations with all reference forms

**Acceptance Criteria**:
- [ ] Session commands parse with real schema/graph references
- [ ] Catalog commands parse with real reference forms
- [ ] All Sprint 4 tests updated to use real references
- [ ] No regressions in existing tests
- [ ] Reference parsing is consistent across all statement types
- [ ] Integration tests validate end-to-end parsing

**File Location**: `src/parser/session.rs`, `src/parser/catalog.rs`, etc.

---

### Task 14: Prepare Integration Points for Future Sprints

**Description**: Document and prepare integration points for future sprint features.

**Deliverables**:
- Document how types will be used in variable declarations (Sprint 7):
  - GRAPH variable :: type = initializer
  - BINDING TABLE variable :: type = initializer
  - VALUE variable :: type = initializer
- Document how types will be used in procedure parameters (Sprint 11)
- Document how types will be used in graph type specifications (Sprint 12):
  - Node type specifications with property types
  - Edge type specifications with property types
  - Endpoint type specifications
- Create placeholder or stub types where detailed specifications deferred:
  - `NestedGraphTypeSpecification` (Sprint 12)
  - `NodeTypeSpecification` (Sprint 12)
  - `EdgeTypeSpecification` (Sprint 12)
- Document type system limitations/extensions for future:
  - User-defined types (if supported)
  - Type inference
  - Type checking (semantic validation in Sprint 14)

**Acceptance Criteria**:
- [ ] Integration points for Sprint 7 (variable declarations) documented
- [ ] Integration points for Sprint 11 (procedure parameters) documented
- [ ] Integration points for Sprint 12 (graph type specifications) documented
- [ ] Placeholder types clearly marked for future implementation
- [ ] Type system architecture documented for future extensions
- [ ] Clear separation between parsing (Sprint 6) and validation (Sprint 14)

**File Location**: `src/ast/types.rs`, `src/parser/types.rs`, documentation

---

### Task 15: Error Recovery and Diagnostics

**Description**: Implement comprehensive error recovery and diagnostic quality for type parsing.

**Deliverables**:
- Error recovery strategies:
  - Recover at type keyword boundaries
  - Recover at comma separators (in field types, parameter lists)
  - Recover at closing delimiters (>, ), })
  - Recover at statement boundaries (propagate up from type parser)
- Diagnostic messages:
  - "Expected type specification, found {token}"
  - "Invalid type parameter: CHAR(n) requires positive integer"
  - "DECIMAL precision {p} must be greater than scale {s}"
  - "Unknown type name '{name}'"
  - "Expected :: or type keyword after expression"
  - "Malformed schema path: expected / or identifier"
  - "Reference parameter must start with $$"
  - "NOT NULL cannot be applied to NULL type"
- Span highlighting for error locations
- Helpful error messages with suggestions:
  - "Did you mean 'INTEGER' instead of 'INTEGR'?"
  - "DOUBLE PRECISION requires both keywords"
  - "TIMESTAMP WITH TIME ZONE, not TIMESTAMP WITH TIMEZONE"

**Grammar References**:
- All type parsing rules (Lines 1713-1998)
- All reference parsing rules (Lines 1379-1478)

**Acceptance Criteria**:
- [ ] Type parser recovers from common errors
- [ ] Multiple errors in one type specification reported
- [ ] Error messages are clear and actionable
- [ ] Span information highlights exact error location
- [ ] Recovery produces sensible partial AST
- [ ] Suggestions provided for common typos
- [ ] Tests validate error recovery behavior

**File Location**: `src/parser/types.rs`, `src/parser/references.rs`, `src/diag.rs`

---

### Task 16: Comprehensive Testing

**Description**: Implement comprehensive test suite for type and reference parsing.

**Deliverables**:

#### Unit Tests (`src/parser/types.rs`):
- **Predefined Type Tests**:
  - Boolean types (BOOL, BOOLEAN)
  - String types (STRING, CHAR, CHAR(10), VARCHAR, VARCHAR(255))
  - Byte string types (BYTES, BINARY, BINARY(16), VARBINARY, VARBINARY(1024))
  - Signed numeric types (INT8, INT16, INT32, INT64, INT128, INT256, SMALLINT, INT, INTEGER, BIGINT)
  - Unsigned numeric types (UINT8, UINT16, UINT32, UINT64, UINT128, UINT256, USMALLINT, UINT, UBIGINT)
  - Decimal types (DECIMAL, DECIMAL(10), DECIMAL(10, 2), DEC, DEC(8, 4))
  - Approximate numeric types (FLOAT16, FLOAT32, FLOAT64, FLOAT128, FLOAT256, FLOAT, FLOAT(53), REAL, DOUBLE PRECISION)
  - Temporal instant types (ZONED DATETIME, LOCAL DATETIME, DATE, ZONED TIME, LOCAL TIME, TIMESTAMP, TIMESTAMP WITH TIME ZONE, TIMESTAMP WITHOUT TIME ZONE, TIME, TIME WITH TIME ZONE, TIME WITHOUT TIME ZONE)
  - Temporal duration types (DURATION, DURATION YEAR TO MONTH, DURATION DAY TO SECOND)
  - Immaterial types (NULL, NULL NOT NULL, NOTHING)

- **Reference Value Type Tests**:
  - Graph reference types (ANY PROPERTY GRAPH, ANY GRAPH, PROPERTY GRAPH, PROPERTY GRAPH <spec>, with/without NOT NULL)
  - Binding table reference types (BINDING TABLE, TABLE, with field types, with/without NOT NULL)
  - Node reference types (NODE, VERTEX, ANY NODE, ANY VERTEX, with/without NOT NULL)
  - Edge reference types (EDGE, RELATIONSHIP, ANY EDGE, ANY RELATIONSHIP, with/without NOT NULL)

- **Constructed Type Tests**:
  - Path types (PATH)
  - List types (LIST<INT>, ARRAY<STRING>, INT LIST, STRING ARRAY, nested lists: LIST<LIST<INT>>)
  - Record types (ANY RECORD, RECORD {}, RECORD { field1 :: INT, field2 :: STRING }, nested records)

- **Type Modifier Tests**:
  - Type annotations (:: INT, TYPED STRING)
  - NOT NULL constraints (INT NOT NULL, STRING NOT NULL, etc.)
  - Combined modifiers (INT NOT NULL, LIST<STRING> NOT NULL)

- **Error Recovery Tests**:
  - Missing type parameters (CHAR(), DECIMAL(,))
  - Invalid type names
  - Malformed type specifications
  - Unclosed delimiters (<, (, {)
  - Invalid NOT NULL placement

#### Unit Tests (`src/parser/references.rs`):
- **Schema Reference Tests**:
  - Absolute paths (/schema, /dir/schema)
  - Relative paths (../schema, ../../other_schema)
  - Predefined references (HOME_SCHEMA, CURRENT_SCHEMA, .)
  - Reference parameters ($$schema_param)

- **Graph Reference Tests**:
  - Catalog-qualified names (schema::graph)
  - Delimited names ("my graph")
  - Home references (HOME_GRAPH, HOME_PROPERTY_GRAPH)
  - Reference parameters ($$graph_param)

- **Other Reference Tests**:
  - Graph type references (schema::graph_type, $$type_param)
  - Binding table references (schema::table, "my table", $$table_param)
  - Procedure references (schema::proc, $$proc_param)

- **Catalog Qualification Tests**:
  - Single-level qualification (schema::name)
  - Multi-level qualification (parent::child::name)
  - Parent reference forms

- **Error Recovery Tests**:
  - Malformed schema paths
  - Invalid reference parameters (single $ instead of $$)
  - Missing :: separator
  - Invalid catalog qualifications

#### Integration Tests (`tests/type_tests.rs` - new file):
- CAST expressions with all type variants
- IS TYPED predicates with all type variants
- Expression :: type annotations
- Session/catalog statements with real references
- Nested type specifications (LIST<RECORD { field :: INT }>)
- Complex reference paths (multi-level catalog qualification)
- Edge cases (deeply nested types, long schema paths)

#### Snapshot Tests:
- Capture AST output for representative types and references
- Ensure AST changes are intentional

**Acceptance Criteria**:
- [ ] >95% code coverage for type and reference parsers
- [ ] All type and reference variants have positive tests
- [ ] Error cases produce clear diagnostics
- [ ] Edge cases covered (empty records, deeply nested types, long paths)
- [ ] Integration tests validate end-to-end parsing
- [ ] Snapshot tests capture AST structure
- [ ] All Sprint 5 CAST/IS TYPED tests work with real types
- [ ] All Sprint 4 reference tests work with real references

**File Location**: `src/parser/types.rs`, `src/parser/references.rs`, `tests/type_tests.rs`

---

### Task 17: Documentation and Examples

**Description**: Document type system and reference parsing with examples.

**Deliverables**:
- **API Documentation**:
  - Rustdoc comments for all type AST node types
  - Rustdoc comments for all reference AST node types
  - Module-level documentation for `src/ast/types.rs`
  - Module-level documentation for `src/ast/references.rs`
  - Module-level documentation for `src/parser/types.rs`
  - Module-level documentation for `src/parser/references.rs`
  - Examples in documentation comments

- **Examples**:
  - Update `examples/parser_demo.rs` to showcase type parsing
  - Add `examples/type_demo.rs` demonstrating:
    - Parsing different predefined types
    - Parsing reference value types
    - Parsing constructed types (list, record, path)
    - Type annotations and modifiers
    - CAST expressions with various types
    - IS TYPED predicates
  - Add `examples/reference_demo.rs` demonstrating:
    - Schema reference parsing
    - Graph reference parsing
    - Catalog-qualified names
    - Reference parameters
    - Session/catalog statements with references

- **Type System Overview Documentation**:
  - Document type hierarchy and categories
  - Document type syntax variants (synonyms, multi-word keywords)
  - Document type parameter semantics (precision, scale, length)
  - Document type modifier semantics (NOT NULL, :: annotation)
  - Cross-reference with ISO GQL specification sections

- **Reference System Overview Documentation**:
  - Document reference forms and contexts
  - Document schema path semantics (absolute, relative, predefined)
  - Document catalog qualification rules
  - Document reference parameter vs value parameter distinction
  - Cross-reference with ISO GQL specification sections

- **Grammar Mapping Documentation**:
  - Document mapping from ISO GQL grammar rules to Rust parser functions
  - Cross-reference with GQL.g4 line numbers
  - Document type parsing precedence and disambiguation rules

- **Error Catalog**:
  - Document all diagnostic codes and messages for types and references
  - Provide examples of each error case
  - Document recovery strategies

**Acceptance Criteria**:
- [ ] All public API documented with rustdoc
- [ ] Examples compile and run successfully
- [ ] Type system overview document complete
- [ ] Reference system overview document complete
- [ ] Grammar mapping document complete
- [ ] Error catalog includes all type/reference error codes
- [ ] Documentation explains type categories clearly
- [ ] Documentation explains reference forms clearly
- [ ] Cross-references to ISO GQL spec provided

**File Location**: `src/ast/types.rs`, `src/ast/references.rs`, `src/parser/types.rs`, `src/parser/references.rs`, `examples/`, `docs/`

---

## Implementation Notes

### Parser Architecture Considerations

1. **Type Parsing Context**: Types appear in many contexts:
   - CAST expressions: `CAST(expr AS type)`
   - IS TYPED predicates: `expr IS [NOT] TYPED type`
   - Variable declarations: `VALUE x :: type = expr`
   - Procedure parameters: `CREATE PROCEDURE p(x :: type)`
   - Graph type specifications: `NODE :Label (prop :: type)`
   - Type parser should be context-agnostic and reusable.

2. **Type vs Expression Disambiguation**: The :: operator appears in both:
   - Type annotation: `expr :: type`
   - Record field type: `field :: type` (in type specifications)
   - Parser must use context to disambiguate
   - In expression context, :: followed by type keyword → type annotation
   - In record type context, :: always means field type

3. **Multi-Word Keywords**: Some types use multi-word keywords:
   - `DOUBLE PRECISION` (two tokens)
   - `TIMESTAMP WITH TIME ZONE` (four tokens)
   - `DURATION YEAR TO MONTH` (four tokens)
   - Parser must handle these as single type units

4. **Type Synonyms**: Many types have synonyms:
   - BOOL / BOOLEAN
   - INT / INTEGER
   - DEC / DECIMAL
   - NODE / VERTEX
   - EDGE / RELATIONSHIP
   - Parser should accept all forms

5. **Parameterized Types**: Some types take parameters:
   - Length: CHAR(n), VARCHAR(n), BINARY(n), VARBINARY(n)
   - Precision: FLOAT(p)
   - Precision/Scale: DECIMAL(p, s), DEC(p, s)
   - Parser must validate parameter presence/absence

6. **NOT NULL Placement**: NOT NULL modifier appears after base type:
   - `INT NOT NULL` (correct)
   - `NOT NULL INT` (incorrect)
   - Parser must enforce correct placement

7. **Reference Parameter vs Value Parameter**:
   - Value parameters: `$name` (used in expressions)
   - Reference parameters: `$$name` (used in catalog references)
   - Lexer must distinguish these token types
   - Parser must use correct parameter type in each context

### AST Design Considerations

1. **Span Tracking**: Every type and reference node must track its source span for diagnostic purposes.

2. **Type Hierarchy**: Use enum hierarchy to represent type categories:
   - `ValueType` (top level)
   - `PredefinedType`, `PathValueType`, `ListValueType`, `RecordType`
   - Further subdivisions within each category
   - This makes pattern matching cleaner and type-safer

3. **Optional Fields**: Many type specifications have optional components:
   - Type parameters: CHAR vs CHAR(10)
   - NOT NULL modifier
   - PROPERTY keyword for graphs
   - Use `Option<T>` appropriately

4. **SmolStr for Efficiency**: Use `SmolStr` for:
   - Type names and keywords (inline storage for short names)
   - Field names in record types
   - Schema path components
   - Reference parameters

5. **Box for Recursion**: Use `Box<ValueType>` for recursive type fields:
   - List element types: `LIST<ValueType>`
   - Record field types: `field :: ValueType`
   - This avoids infinite size types

6. **Placeholder Types**: Sprint 6 includes reference value types that reference graph type specifications:
   - `PROPERTY GRAPH <nested_graph_type_specification>`
   - `NODE <node_type_specification>`
   - `EDGE <edge_type_specification>`
   - Use placeholder types for these, to be fully implemented in Sprint 12

### Reference Parsing Considerations

1. **Schema Path Parsing**: Schema paths use special syntax:
   - Absolute: `/schema` or `/dir/schema`
   - Relative: `../schema` or `../../other/schema`
   - Path components separated by `/`
   - Parser must handle path traversal semantics

2. **Catalog Qualification**: Qualified names use `::` separator:
   - `schema::graph`
   - `parent::child::name`
   - Parser must handle multi-level qualification

3. **Delimited Identifiers**: Some references allow delimited identifiers:
   - `"my graph with spaces"`
   - Parser must distinguish from string literals (context-dependent)

4. **Predefined References**: Some references are keywords:
   - HOME_SCHEMA, CURRENT_SCHEMA
   - HOME_GRAPH, HOME_PROPERTY_GRAPH
   - Parser must recognize these as special forms

### Error Recovery Strategy

1. **Synchronization Points**:
   - Type keyword boundaries (when parsing type, stop at non-type keyword)
   - Comma separators (in field types, parameter lists)
   - Closing delimiters (>, ), })
   - Statement keywords (propagate up to statement parser)

2. **Type Parameter Recovery**: If type parameter malformed:
   - Report error at parameter location
   - Use default or omit parameter
   - Continue parsing rest of type

3. **Reference Path Recovery**: If schema path malformed:
   - Report error at invalid component
   - Use partial path or default reference
   - Continue parsing rest of statement

4. **Delimiter Matching**: Track opening delimiters (<, (, {) and ensure closing:
   - Use stack for nested delimiters (e.g., LIST<RECORD { field :: INT }>)
   - Report unclosed delimiter errors with helpful span

### Diagnostic Quality Guidelines

1. **Clear Messages**: Error messages should be specific:
   - Bad: "Parse error in type"
   - Good: "DECIMAL precision must be at least 1, found 0"

2. **Helpful Suggestions**:
   - "Did you mean 'INTEGER' instead of 'INTEGR'?"
   - "DOUBLE PRECISION requires both keywords, not just DOUBLE"
   - "Use $$ for reference parameters, not $ (value parameters)"
   - "Schema path must start with / (absolute) or .. (relative)"

3. **Span Highlighting**: Highlight the exact token or range causing the error:
   - For missing type parameters, point to where parameter expected
   - For malformed schema paths, highlight invalid component
   - For type mismatches, highlight conflicting types

4. **Context Information**: Provide context about what parser was doing:
   - "While parsing DECIMAL type parameters..."
   - "In schema path starting at line 42..."
   - "While parsing field type specification for record..."

### Performance Considerations

1. **Type Keyword Recognition**: Type keywords are common, so recognition must be fast:
   - Use efficient keyword table (trie or perfect hash)
   - Cache commonly-used type AST nodes (if beneficial)

2. **Type Parsing Efficiency**: Type parsing is hot path:
   - Minimize lookahead (most types identifiable by first keyword)
   - Avoid excessive backtracking
   - Use direct dispatch to type-specific parsers

3. **Reference Resolution**: Reference parsing is frequent:
   - Optimize schema path parsing (common case: simple names)
   - Cache catalog-qualified name parsing logic
   - Minimize allocations for path components

4. **AST Allocation**: Minimize allocations:
   - Use `Box` only where needed for recursion
   - Use `SmolStr` for inline storage of short strings
   - Consider arena allocation for AST nodes (future optimization)

## Dependencies on Other Sprints

### Dependencies on Completed Sprints

- **Sprint 1**: Diagnostic infrastructure (`Span`, `Diag`, `miette` conversion)
- **Sprint 2**: Lexer emits all necessary tokens (type keywords, ::, $$, /, ..)
- **Sprint 3**: Parser skeleton and recovery framework
- **Sprint 4**: Statement structure for integration testing; catalog reference placeholders to replace
- **Sprint 5**: Expression parsing for CAST and IS TYPED integration; CastExpression and Predicate AST to update

### Dependencies on Future Sprints

- **Sprint 7**: Query clauses will use type annotations in variable declarations (LET, FOR with :: type)
- **Sprint 11**: Procedure definitions will use types for parameter specifications
- **Sprint 12**: Graph type specifications will use types for property definitions in node/edge type specs
- **Sprint 14**: Semantic validation will use type AST for type checking

### Cross-Sprint Integration Points

- Types are foundational and will be used throughout all future sprints
- Type parser must be designed for reusability across all contexts
- AST type definitions should be stable to avoid downstream breakage
- Reference parsers replace Sprint 4 placeholder reference types
- Consider semantic validation in Sprint 14 (type compatibility, type checking, etc.)

## Test Strategy

### Unit Tests

For each type and reference component:
1. **Happy Path**: Valid types/references parse correctly
2. **Variants**: All syntax variants and optional components
3. **Error Cases**: Missing parameters, invalid syntax, malformed specifications
4. **Recovery**: Parser recovers and continues after errors

### Integration Tests

Types and references in different contexts:
1. **CAST Expressions**: All type variants in CAST(expr AS type)
2. **IS TYPED Predicates**: All type variants in IS TYPED type
3. **Session Commands**: All reference forms in SESSION SET SCHEMA/GRAPH
4. **Catalog Operations**: All reference forms in CREATE/DROP statements
5. **Nested Types**: Deeply nested type specifications (LIST<RECORD<LIST<INT>>>)
6. **Complex References**: Multi-level catalog qualification (parent::child::name)

### Snapshot Tests

Capture AST output:
1. Representative types from each category
2. Complex nested types
3. All reference forms
4. Ensure AST changes are intentional

### Property-Based Tests (Optional, Advanced)

Use `proptest` or `quickcheck`:
1. Generate random valid types
2. Verify parser never panics
3. Verify parsed AST can be pretty-printed and re-parsed

### Corpus Tests

Test against real GQL queries (from future sprints):
1. Identify queries with complex type annotations
2. Verify parser handles real-world type syntax

### Performance Tests

1. **Deeply Nested Types**: Ensure parser handles deep nesting efficiently (LIST<LIST<LIST<...>>>)
2. **Long Schema Paths**: Schema paths with many components
3. **Complex Type Specifications**: Types with multiple modifiers and parameters

## Performance Considerations

1. **Lexer Efficiency**: Type keywords are frequent; lexer must be fast
2. **Parser Efficiency**: Use direct dispatch to type-specific parsers (no backtracking)
3. **AST Allocation**: Minimize allocations with `SmolStr` and arena allocation (future)
4. **Type Keyword Lookup**: Table-driven for constant-time lookup

## Documentation Requirements

1. **API Documentation**: Rustdoc for all type and reference AST nodes and parser functions
2. **Type System Overview**: Document type hierarchy and categories
3. **Reference System Overview**: Document reference forms and contexts
4. **Grammar Mapping**: Document ISO GQL grammar to Rust parser mapping
5. **Examples**: Demonstrate type and reference parsing in examples
6. **Error Catalog**: Document all diagnostic codes and messages

## Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Type grammar complexity causes parser confusion | High | Medium | Careful grammar analysis; extensive testing; clear AST design |
| Multi-word keyword ambiguity | Medium | Low | Use multi-token lookahead; document ambiguous cases |
| Type vs expression disambiguation complexity (::) | High | Medium | Use context to guide parsing; clear separation of concerns |
| Reference parameter vs value parameter confusion | Medium | Low | Lexer distinguishes token types; clear documentation |
| Placeholder type specifications limit testing | Medium | Medium | Document placeholders clearly; defer full specs to Sprint 12 |
| NOT NULL placement complexity | Low | Low | Enforce grammar rules in parser; clear error messages |
| Type synonym coverage | Low | Medium | Comprehensive testing; document all synonym forms |

## Success Metrics

1. **Coverage**: All type and reference forms parse with correct AST
2. **Correctness**: Type syntax matches ISO GQL specification (validated by tests)
3. **Diagnostics**: At least 90% of error cases produce helpful diagnostics
4. **Recovery**: Parser never panics on invalid input
5. **Test Coverage**: >95% code coverage for type and reference parsers
6. **Performance**: Parser handles types with 100+ nested levels in <1ms
7. **Integration**: Type parser integrates cleanly with Sprint 5 (expressions) and Sprint 4 (statements)
8. **Reusability**: Type parser used in multiple contexts (CAST, IS TYPED, variables, etc.)

## Sprint Completion Checklist

- [ ] All tasks completed and reviewed
- [ ] All acceptance criteria met
- [ ] Unit tests pass with >95% coverage
- [ ] Integration tests demonstrate end-to-end functionality
- [ ] Documentation complete (rustdoc, examples, grammar mapping, type system overview)
- [ ] Performance baseline established
- [ ] Error catalog documented
- [ ] Code review completed
- [ ] CI/CD pipeline passes
- [ ] Type parser tested in multiple contexts (CAST, IS TYPED, statements)
- [ ] Reference parser tested in multiple contexts (session, catalog operations)
- [ ] AST design reviewed for stability and extensibility
- [ ] Placeholder types for Sprint 12 documented clearly
- [ ] Sprint 5 integration complete (CAST, IS TYPED use real types)
- [ ] Sprint 4 integration complete (statements use real references)
- [ ] Sprint demo prepared for stakeholders

## Next Sprint Preview

**Sprint 7: Query Pipeline Core** will build on the type foundation to implement linear/composite query composition and clause chaining (MATCH, FILTER, LET, FOR, SELECT). With types implemented, Sprint 7 can focus on variable declarations with type annotations and query result typing.

---

## Appendix: Type System Hierarchy

```
ValueType
├── PredefinedType
│   ├── Boolean (BOOL, BOOLEAN)
│   ├── CharacterString (STRING, CHAR, VARCHAR)
│   ├── ByteString (BYTES, BINARY, VARBINARY)
│   ├── Numeric
│   │   ├── Exact
│   │   │   ├── SignedBinary (INT8, INT16, INT32, INT64, INT128, INT256, SMALLINT, INT, BIGINT)
│   │   │   ├── UnsignedBinary (UINT8, UINT16, UINT32, UINT64, UINT128, UINT256, USMALLINT, UINT, UBIGINT)
│   │   │   └── Decimal (DECIMAL, DEC)
│   │   └── Approximate (FLOAT16, FLOAT32, FLOAT64, FLOAT128, FLOAT256, FLOAT, REAL, DOUBLE PRECISION)
│   ├── Temporal
│   │   ├── Instant
│   │   │   ├── ZonedDatetime (ZONED DATETIME, TIMESTAMP WITH TIME ZONE)
│   │   │   ├── LocalDatetime (LOCAL DATETIME, TIMESTAMP WITHOUT TIME ZONE)
│   │   │   ├── Date (DATE)
│   │   │   ├── ZonedTime (ZONED TIME, TIME WITH TIME ZONE)
│   │   │   └── LocalTime (LOCAL TIME, TIME WITHOUT TIME ZONE)
│   │   └── Duration
│   │       ├── Duration (DURATION)
│   │       ├── DurationYearToMonth (DURATION YEAR TO MONTH)
│   │       └── DurationDayToSecond (DURATION DAY TO SECOND)
│   ├── ReferenceValue
│   │   ├── Graph (ANY PROPERTY GRAPH, PROPERTY GRAPH <spec>)
│   │   ├── BindingTable (BINDING TABLE <field_types>)
│   │   ├── Node (ANY NODE, VERTEX, <node_type_spec>)
│   │   └── Edge (ANY EDGE, RELATIONSHIP, <edge_type_spec>)
│   └── Immaterial (NULL, NULL NOT NULL, NOTHING)
├── PathValue (PATH)
├── ListValue (LIST<T>, ARRAY<T>, T LIST, T ARRAY)
└── Record (ANY RECORD, RECORD { field :: type, ... })

Type Modifiers:
- :: type (type annotation)
- TYPED type (type annotation keyword form)
- NOT NULL (non-nullable constraint)
```

---

## Appendix: Reference System Forms

```
References:
├── SchemaReference
│   ├── AbsolutePath: /schema, /dir/schema
│   ├── RelativePath: ../schema, ../../other/schema
│   ├── Predefined: HOME_SCHEMA, CURRENT_SCHEMA, .
│   └── ReferenceParameter: $$schema_param
├── GraphReference
│   ├── CatalogQualified: schema::graph
│   ├── Delimited: "my graph"
│   ├── Predefined: HOME_GRAPH, HOME_PROPERTY_GRAPH
│   └── ReferenceParameter: $$graph_param
├── GraphTypeReference
│   ├── CatalogQualified: schema::graph_type
│   └── ReferenceParameter: $$type_param
├── BindingTableReference
│   ├── CatalogQualified: schema::table
│   ├── Delimited: "my table"
│   └── ReferenceParameter: $$table_param
└── ProcedureReference
    ├── CatalogQualified: schema::procedure
    └── ReferenceParameter: $$proc_param

Catalog Qualification:
- Single-level: schema::name
- Multi-level: parent::child::name
- Parent reference: SchemaReference or nested CatalogQualifiedName
```

---

## Appendix: Type Grammar Coverage Map

| Grammar Rule | Line Number | AST Node | Parser Function |
|--------------|-------------|----------|-----------------|
| `valueType` | 1719 | `ValueType` enum | `parse_value_type()` |
| `predefinedType` | 1740 | `PredefinedType` enum | `parse_predefined_type()` |
| `booleanType` | 1750 | `BooleanType` enum | `parse_boolean_type()` |
| `characterStringType` | 1754 | `CharacterStringType` enum | `parse_character_string_type()` |
| `byteStringType` | 1760 | `ByteStringType` enum | `parse_byte_string_type()` |
| `numericType` | 1778 | `NumericType` enum | `parse_numeric_type()` |
| `exactNumericType` | 1783 | `ExactNumericType` enum | `parse_exact_numeric_type()` |
| `signedBinaryExactNumericType` | 1793 | `SignedBinaryExactNumericType` enum | `parse_signed_binary_exact_numeric_type()` |
| `unsignedBinaryExactNumericType` | 1806 | `UnsignedBinaryExactNumericType` enum | `parse_unsigned_binary_exact_numeric_type()` |
| `decimalExactNumericType` | 1831 | `DecimalExactNumericType` struct | `parse_decimal_exact_numeric_type()` |
| `approximateNumericType` | 1843 | `ApproximateNumericType` enum | `parse_approximate_numeric_type()` |
| `temporalType` | 1854 | `TemporalType` enum | `parse_temporal_type()` |
| `temporalInstantType` | 1859 | `TemporalInstantType` enum | `parse_temporal_instant_type()` |
| `temporalDurationType` | 1891 | `TemporalDurationType` enum | `parse_temporal_duration_type()` |
| `referenceValueType` | 1900 | `ReferenceValueType` enum | `parse_reference_value_type()` |
| `graphReferenceValueType` | 1921 | `GraphReferenceValueType` enum | `parse_graph_reference_value_type()` |
| `bindingTableReferenceValueType` | 1934 | `BindingTableReferenceValueType` struct | `parse_binding_table_reference_value_type()` |
| `nodeReferenceValueType` | 1938 | `NodeReferenceValueType` enum | `parse_node_reference_value_type()` |
| `edgeReferenceValueType` | 1951 | `EdgeReferenceValueType` enum | `parse_edge_reference_value_type()` |
| `immaterialValueType` | 1907 | `ImmaterialValueType` enum | `parse_immaterial_value_type()` |
| `pathValueType` | 1964 | `PathValueType` struct | `parse_path_value_type()` |
| `listValueTypeName` | 1968 | `ListValueType` struct | `parse_list_value_type()` |
| `recordType` | 1977 | `RecordType` enum | `parse_record_type()` |
| `fieldTypesSpecification` | 1982 | `FieldTypesSpecification` struct | `parse_field_types_specification()` |
| `fieldType` | 1996 | `FieldType` struct | `parse_field_type()` |
| `typed` | 1735 | `TypeAnnotation` struct | `parse_type_annotation()` |
| `notNull` | 1990 | `NotNullConstraint` struct | `parse_not_null_constraint()` |

---

## Appendix: Reference Grammar Coverage Map

| Grammar Rule | Line Number | AST Node | Parser Function |
|--------------|-------------|----------|-----------------|
| `schemaReference` | 1381 | `SchemaReference` enum | `parse_schema_reference()` |
| `graphReference` | 1421 | `GraphReference` enum | `parse_graph_reference()` |
| `graphTypeReference` | 1439 | `GraphTypeReference` enum | `parse_graph_type_reference()` |
| `bindingTableReference` | 1450 | `BindingTableReference` enum | `parse_binding_table_reference()` |
| `procedureReference` | 1458 | `ProcedureReference` enum | `parse_procedure_reference()` |
| `catalogObjectParentReference` | 1469 | `CatalogObjectParentReference` enum | `parse_catalog_object_parent_reference()` |

---

**Document Version**: 1.0
**Date Created**: 2026-02-17
**Status**: Planned
**Dependencies**: Sprints 1, 2, 3, 4 (completed), Sprint 5 (required)
**Next Sprint**: Sprint 7 (Query Pipeline Core)
