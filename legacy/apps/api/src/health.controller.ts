import { Controller, Get } from "@nestjs/common";
import { PrismaService } from "./common/prisma.service.js";

@Controller()
export class HealthController {
  constructor(private readonly prisma: PrismaService) {}

  @Get("health")
  async health() {
    // 轻量存活探测：DB 走一次 SELECT 1
    await this.prisma.$queryRaw`SELECT 1`;
    return { data: { status: "ok", ts: new Date().toISOString() } };
  }
}
