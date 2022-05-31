## 自动维护 /etc/hosts 文件域名对应ip可用性程序
### 已完成功能
- 维护单个域名下对应ip的可靠性，周期性尝试hosts文件中绑定域名所对应ip的延时情况，多次超时后自动切换

### 未完成功能
- 同时维护多个域名对应ip可靠性
- 域名ip绑定周期切换
- host 编辑 gui 功能

### 编译执行
```
cargo build --release

chown root target/release/hosts_random
chmod a+s target/release/hosts_random
target/release/hosts_random --host ${youhostname}.com
```