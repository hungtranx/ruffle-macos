# Plan Fix Memory Retention Trong Ruffle

## Summary

- Vấn đề chính cần xử lý trước là Ruffle chưa implement đúng flash.utils.Dictionary(true).
- Hiện tại weak-key dictionary vẫn giữ object key như strong reference, nên nếu game dùng dictionary này cho cache map/display/
  resource, object graph cũ có thể không được GC.
- Hướng đầu tiên là sửa đúng semantics của weak-key dictionary trong Ruffle core. Chưa ưu tiên Loader.unload() hoặc graphics
  cleanup cho đến khi loại trừ retention từ weak dictionary.

## Vấn Đề

- new Dictionary(true) trong Flash nghĩa là object keys không được giữ sống chỉ vì dictionary còn reference tới key.
- Trong Ruffle hiện tại, constructor weakKeys=true chỉ là stub.
- Object keys trong Dictionary đang được lưu như dynamic object properties mạnh.
- Điều này có thể giữ lại display objects, AVM2 objects, BitmapData hoặc resource graph của scene/map cũ sau khi game chuyển map.

## Hướng Giải Quyết

- Implement trạng thái weakKeys thật cho flash.utils.Dictionary.
- Với dictionary thường, giữ nguyên behavior hiện tại.
- Với weak dictionary, các object key phải được lưu bằng weak reference.
- Khi key object đã bị GC, entry tương ứng phải biến mất khỏi lookup, in, delete và enumeration.
- String/literal keys không cần đổi, vì weak-key behavior chỉ áp dụng cho object keys.
- Sau khi hoàn tất, bỏ stub warning cho Dictionary(true).

## Kiểm Chứng

- Chạy lại các regression test hiện có quanh Dictionary.
- Thêm test cho weak dictionary: object key chỉ còn được dictionary giữ thì phải collect được.
- Kiểm tra for in, for each, lookup, delete và in vẫn đúng khi key còn sống và khi key đã chết.
- Sau đó chạy lại native diagnostics với game để xem memory có còn tăng theo từng map transition không.

## Phase Sau

- Nếu memory vẫn tăng, chuyển sang Loader.unload() / unloadAndStop().
- Sau đó mới điều tra graphics/Metal resource cleanup, vì graphics growth có thể chỉ là hệ quả của AVM2/display object retention.



› viết vào file PLAN.md ở /Users/eklipse/ppro/ruffle-macos


• Hiện đang ở Plan Mode nên tôi chưa được ghi file trực tiếp. Nội dung nên đưa vào /Users/eklipse/ppro/ruffle-macos/PLAN.md là:

# Plan Fix Memory Retention Trong Ruffle

## Summary

Vấn đề đầu tiên cần xử lý là `flash.utils.Dictionary(true)` chưa có weak-key behavior thật trong Ruffle. Nếu game dùng weak
dictionary để cache map, display object hoặc resource, Ruffle hiện có thể giữ object graph cũ sống quá lâu, làm memory tăng sau mỗi
lần chuyển map.

## Vấn Đề

Trong Flash, `new Dictionary(true)` không giữ object keys sống chỉ vì dictionary còn reference tới key.

Trong Ruffle hiện tại:

- `weakKeys=true` vẫn chỉ là stub.
- Object keys vẫn được lưu như strong references.
- Điều này có thể giữ lại AVM2 objects, display objects, BitmapData hoặc texture-related resources của scene/map cũ.

## Hướng Giải Quyết

Implement weak-key semantics thật cho `flash.utils.Dictionary`.

- Dictionary thường giữ behavior hiện tại.
- Weak dictionary lưu object keys bằng weak reference.
- Khi key object đã bị GC, entry tương ứng không còn xuất hiện trong lookup, `in`, delete hoặc enumeration.
- String/literal keys giữ nguyên behavior hiện tại.
- Sau khi implement, bỏ stub warning cho `Dictionary(true)`.

## Kiểm Chứng

- Chạy lại regression tests quanh `Dictionary`.
- Thêm test cho weak dictionary: object key chỉ còn được dictionary giữ thì phải collect được.
- Kiểm tra lookup, delete, `in`, `for in`, `for each` khi key còn sống và sau khi key chết.
- Sau đó chạy lại native diagnostics với game để xem memory có còn tăng theo từng map transition không.

## Phase Sau

Nếu memory vẫn tăng sau weak dictionary fix:

1. Điều tra `Loader.unload()` / `unloadAndStop()`.
2. Kiểm tra lifecycle event và cleanup của loaded content.
3. Sau đó mới đi sâu vào graphics/Metal resource cleanup.
