# Orderbook Component Integration Plan

This document outlines the steps needed to fully integrate the orderbook component into the Astria sequencer.

## 1. Protocol Buffer Integration

### Current Status
- We have basic protobuf definitions in `/proto/protocolapis/astria/protocol/orderbook/v1/`
- We created templates for transaction action and fees integration

### Next Steps
1. Finalize the transaction action.proto to include orderbook actions
2. Finalize the fees types.proto to include orderbook fees
3. Generate Rust code from the protobuf definitions
4. Update our compatibility layer to use the generated Rust code

## 2. Storage Integration

### Current Status
- Added Market and Trade to StoredValue enum
- Added Bytes variant for generic serialized data
- Added Key structure for hierarchical key management
- Added additional key functions for orderbook-specific keys

### Next Steps
1. Update state_ext.rs to use StoredValue and Key properly
2. Ensure proper serialization/deserialization of our types
3. Add proper error handling for storage operations

## 3. Component Registration

### Current Status
- OrderbookComponent is defined but not registered in the app
- Basic implementation is present but needs to be completed

### Next Steps
1. Complete the Component trait implementation
2. Update app/mod.rs to call init_chain, begin_block, end_block on the orderbook component
3. Add proper error handling for component operations

## 4. Action Handling

### Current Status
- We have created checked action definitions
- Basic action handlers are present

### Next Steps
1. Complete action_ref.rs to include orderbook actions
2. Implement action registration for orderbook actions
3. Add proper fee handling for orderbook actions
4. Add validation for all orderbook actions
5. Connect the orderbook component to the transaction execution flow

## 5. Query Integration

### Current Status
- Basic query handlers are present but need to be updated

### Next Steps
1. Complete query.rs to handle all orderbook queries
2. Update query routing to include orderbook queries
3. Add proper error handling for query operations
4. Implement pagination for large result sets

## 6. Testing

### Current Status
- Basic unit tests for matching engine
- No integration tests

### Next Steps
1. Expand unit tests for all orderbook operations
2. Add integration tests for order placement and matching
3. Test transaction execution with orderbook actions
4. Test all query endpoints
5. Add performance tests for matching engine

## 7. Documentation

### Current Status
- Basic README.md
- NEXT_STEPS.md

### Next Steps
1. Add full API documentation
2. Document protobuf schemas
3. Create examples of using the orderbook component
4. Add architecture documentation
5. Add operational documentation

## Implementation Timeline

### Phase 1: Core Infrastructure (2 weeks)
- Complete storage integration
- Update Key and StoredValue handling
- Complete basic Component implementation

### Phase 2: Transaction Flow (2 weeks)
- Complete action handling
- Complete fee handling
- Integrate with transaction execution flow

### Phase 3: Query and API (2 weeks)
- Complete query handlers
- Add proper error handling
- Implement pagination

### Phase 4: Testing and Documentation (2 weeks)
- Add comprehensive tests
- Complete documentation
- Create examples