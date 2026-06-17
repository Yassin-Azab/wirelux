// ── libpcap ──
#include <pcap/pcap.h>

// ── Packet structure structs ──
#include <net/ethernet.h>      // struct ethhdr, ETH_P_IP
#include <netinet/ip.h>        // struct iphdr, IPPROTO_TCP, IPPROTO_UDP
#include <netinet/tcp.h>       // struct tcphdr
#include <netinet/udp.h>       // struct udphdr

// ── IP and byte-order utilities ──
#include <arpa/inet.h>         // ntohs(), ntohl(), inet_ntop(), INET_ADDRSTRLEN

// ── DNS resolution ──
#include <netdb.h>             // getnameinfo(), NI_MAXHOST
#include <sys/socket.h>        // AF_INET, struct sockaddr
#include <netinet/in.h>        // struct sockaddr_in

// ── /proc directory traversal ──
#include <dirent.h>            // opendir(), readdir(), closedir(), struct dirent
#include <unistd.h>            // readlink()

// ── Signal handling ──
#include <csignal>             // signal(), SIGINT
#include <iostream>
#include <chrono> // Required for modern time handling
// ── C++ standard library ──
#include <fstream>
#include <sstream>

struct AppStats {
    uint64_t    bytes_sent     =0;
    uint64_t    bytes_received =0;
    uint32_t   src_ip=0;
    uint32_t   dst_ip=0;
    bool isOutbound=NULL;
    uint64_t total() const {
        return bytes_sent + bytes_received;
    }
};

pcap_t* g_handle = nullptr;  
using TimePoint = std::chrono::time_point<std::chrono::system_clock>;
std::map<std::string, std::pair<AppStats, TimePoint>> AppData;

void packethandler(u_char* user, const struct pcap_pkthdr* header, const u_char* packet){
if (header->caplen < 34) return;
auto* app_data = reinterpret_cast<std::map<std::string, std::pair<AppStats, TimePoint>>*>(user);
const struct ethhdr* eth =reinterpret_cast<const struct ethhdr*>(packet);
if (ntohs(eth->h_proto) != ETH_P_IP) return;
const struct iphdr* ip = reinterpret_cast<const struct iphdr*>(packet + sizeof(struct ethhdr));
if ((ip->protocol != IPPROTO_TCP && ip->protocol != IPPROTO_UDP)|| ip->version != 4||ip->ihl < 5||(header->caplen < sizeof(struct ethhdr) + ip->ihl*4)) return;
uint32_t src_ip   = ntohl(ip->saddr); 
uint32_t dst_ip   = ntohl(ip->daddr);
uint16_t src_port = 0, dst_port = 0;
bool isOutbound = false;
const struct u_char* transport = packet + 14 + ip->ihl*4;

if (ip->protocol == IPPROTO_TCP) { 
const struct tcphdr* tcp = reinterpret_cast<const struct tcphdr*>(transport);
src_port = ntohs(tcp->source);
dst_port = ntohs(tcp->dest);
auto [eval, output]=find_Outbound_by_port(src_port,dst_port, true);
if(output) isOutbound=eval;

}

if (ip->protocol == IPPROTO_UDP) { 
const struct udphdr* udp = reinterpret_cast<const struct udphdr*>(transport);
src_port = ntohs(udp->source);
dst_port = ntohs(udp->dest);
auto [eval, output]=find_Outbound_by_port(src_port,dst_port, false);
if(output) isOutbound=eval;
}

}



std::pair<bool, bool> find_Outbound_by_port(uint16_t src_port,uint16_t dst_port,bool isTcp){

std::string path =isTcp ? "/proc/net/tcp" : "/proc/net/udp";
std::ifstream file(path);
if (!file.is_open()) {
    std::cerr << "Error opening " << path << std::endl;
    return std::make_pair(false, false);
}

std::string line;
std::getline(file, line);
while (std::getline(file, line)) {
    std::istringstream iss(line);
    std::string slot, local_addr, rem_addr, state,txrx, trwhen, retrnsmt;
    unsigned long uid, timeout, inode;
    iss >> slot
        >> local_addr
        >> rem_addr
        >> state
        >> txrx
        >> trwhen
        >> retrnsmt
        >> uid
        >> timeout
        >> inode;
    size_t Local_colon_pos = local_addr.find(':');
    size_t Rem_colon_pos = rem_addr.find(':');
    if (Local_colon_pos != std::string::npos && Rem_colon_pos != std::string::npos) {
        std::string local_port_hex = local_addr.substr(Local_colon_pos + 1);
        std::string rem_port_hex = rem_addr.substr(Rem_colon_pos + 1);
        uint16_t local_port = static_cast<uint16_t>(std::stoul(local_port_hex, nullptr, 16));
        uint16_t rem_port = static_cast<uint16_t>(std::stoul(rem_port_hex, nullptr, 16));
        if (local_port == src_port && rem_port == dst_port) {
            return std::make_pair(true, true);
        }
        else if(local_port == dst_port && rem_port == src_port) {
            //add to cache
            return std::make_pair(false, true);
        }
    }

}
return std::make_pair(false, false);
}
void on_ctrl_c(int) {
    if (g_handle) {pcap_breakloop(g_handle);}
}

int main(){

pcap_if_t* AllInterfaces = nullptr;
char InterfaceErrorrs[PCAP_ERRBUF_SIZE];
if (pcap_findalldevs(pcap_if_t **AllInterfaces, char *InterfaceErrorrs) == -1) {
    std::cerr << "Error finding interfaces: " << InterfaceErrorrs << std::endl;
    return 1;
}
int i=1;
pcap_if_t* chosenInt=nullptr;
for (pcap_if_t* CurrInt = AllInterfaces;CurrInt!=nullptr ;CurrInt=(*CurrInt).next) {
    if(!(CurrInt->flags & PCAP_IF_LOOPBACK)&& CurrInt ->flags & PCAP_IF_RUNNING && (dev->flags & PCAP_IF_CONNECTION_STATUS) == PCAP_IF_CONNECTION_STATUS_CONNECTED) && std::string(CurrInt->name) != "any" ;
        {std::string line =std::format("{}. Interface Name: {}", i, (*CurrInt).name);
        std::cout << line << std::endl;
        ++i;
        if (chosenInt==nullptr && (CurrInt->name).std::string::contains(wl))
            chosenInt=CurrInt;
        else if (chosenInt==nullptr && CurrInt->next==nullptr)
            chosenInt=CurrInt;
        }
}

//std::chrono::auto startTime = std::chrono::steady_clock::now(); + seconds(60);
g_handle =pcap_open_live(chosenInt->name, 65536, 1, 1000, InterfaceErrorrs);
if (g_handle == nullptr) {
    std::cerr << "Error opening device: " << InterfaceErrorrs << std::endl;
    return 2;
}

int pcap_loop(g_handle, -1, &packethandler, reinterpret_cast<u_char*>(&AppData));
signal(SIGINT, on_ctrl_c);
void pcap_breakloop(pcap_t *handle);
void pcap_freealldevs(pcap_if_t *AllInterfaces);
pcap_close(g_handle);
AllInterfaces=nullptr;
g_handle = nullptr;
return 0;
}
