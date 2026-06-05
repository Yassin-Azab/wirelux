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

// ── C++ standard library ──
#include <iostream>            // std::cout, std::cerr
#include <fstream>             // std::ifstream
#include <sstream>             // std::istringstream
#include <string>              // std::string, std::to_string, std::stoul, std::stoi
#include <map>                 // std::map
#include <cstring>             // memset()
#include <cstdint>             // uint8_t, uint16_t, uint32_t, uint64_t