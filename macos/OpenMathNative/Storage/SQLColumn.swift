import Foundation
import SQLite3

enum SQLColumn {
  static func data(_ row:OpaquePointer,_ column:Int32,maximumBytes:Int=4*1024*1024) throws ->Data {
    let count=Int(sqlite3_column_bytes(row,column))
    guard maximumBytes>0,maximumBytes<=8*1024*1024,count>=0,count<=maximumBytes else { throw StorageError.corruptIdentity }
    if count==0 { return Data() }
    guard let bytes=sqlite3_column_blob(row,column) else { throw StorageError.corruptIdentity }
    return Data(bytes:bytes,count:count)
  }
  static func text(_ row:OpaquePointer,_ column:Int32) throws ->String {
    guard let value=String(data:try data(row,column),encoding:.utf8) else { throw StorageError.corruptIdentity }
    return value
  }
}
